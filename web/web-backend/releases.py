import asyncio, json, logging
from time import monotonic
from urllib.error import URLError
from urllib.parse import urlsplit
from urllib.request import Request, urlopen

from config import (success_cache_seconds, error_cache_seconds, 
error_github_timeout, error_max_release_size, Github_Release_API, Installer_Name)

from schemas import ReleaseInfo

logger = logging.getLogger(__name__)

# Asks Github for latest published release of Molten Cheese
def fetch_github_release() -> dict:
    request = Request(Github_Release_API, headers={
        "Accept": "application/vnd.github+json",
        "X-Github-Api-Version": "2022-11-28",
        "User-Agent": "Molten-Cheese-Website"
    })

    with urlopen(request, timeout=error_github_timeout) as response:
        raw_data=response.read(error_max_release_size+1)

    if len(raw_data) > error_max_release_size:
        raise ValueError ("Github response size is too large")

    release = json.loads(raw_data)

    if not isinstance(release, dict):
        raise ValueError("Invalid release response")

    if release.get("draft") is not False:
        raise ValueError("Release isn't published yet")

    if release.get("prerelease") is not False:
        raise ValueError("Release isn't stable")

    return release

# Finds the installer in Github
def select_installer_asset(release: dict) -> dict:
    assets = release.get("assets")

    if not isinstance(assets, list):
        raise ValueError("Invaled asset list")

    matches = [
        asset
        for asset in assets if isinstance(asset,dict)
        and asset.get("name") == Installer_Name
        and asset.get("state") == "uploaded"
    ]

    if len(matches) != 1:
        raise ValueError("Expected installer not found")

    return matches[0]

# Checks that download link belong to Molten Cheese's releases n not anything else
def validate_download_url(value: object) -> str:
    if not isinstance(value, str):
        raise ValueError("Download URL must be a text")

    if any(character.isspace() for character in value):
        raise ValueError("Download URL contains whitespace")

    url = urlsplit(value)

    ## Molten Cheese releases location at Github
    expected_path = ("/dylannajib/Molten-Cheese/releases/download/")

    if (
        url.scheme != "https"
        or url.netloc != "github.com"
        or not url.path.startswith(expected_path)
        or not url.path.endswith("/" + Installer_Name)
        or url.query
        or url.fragment
    ):
        raise ValueError ("Invalid download URL")

    return value

# Shrink Github's data for our website
def build_release_info(
    release: dict,
    asset: dict
) -> ReleaseInfo:
    version = release.get("tag_name")
    notes = release.get("body")
    size = asset.get("size")

    if not isinstance(version, str) or not version.strip():
        raise ValueError("Invalid release version")

    if notes is None:
        notes = ""

    if not isinstance(notes, str):
        raise ValueError("Invalid release notes")

    if type(size) is not int or size <= 0:
        raise ValueError("invalid installer size")

    download_url = validate_download_url(asset.get("browser_download_url"))

    return ReleaseInfo(
        status="available",
        version=version,
        file_size=size,
        release_notes=notes,
        download_url=download_url
    )

# Reuses recent release info instead of always asking Github
async def get_cached_release(cache: dict) -> ReleaseInfo:
    if monotonic() < cache["expires_at"]:
        return cache["release"]
    
    try:
        ## Make network wait outside the server's main event loop
        release = await asyncio.to_thread(fetch_github_release)

        asset = select_installer_asset(release)
        result = build_release_info(release, asset)

        lifetime = success_cache_seconds
    except (URLError, OSError, ValueError):
        logger.warning("Release info couldn't be retrieved or validated")

        result = ReleaseInfo(status="unavailable")
        lifetime = error_cache_seconds

    cache["release"] = result
    cache["expires_at"] = monotonic() + lifetime

    return result