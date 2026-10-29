from pydantic import BaseModel, ConfigDict

Github_Release_API = (
    "https://api.github.com/repos/"
    "dylannajib/Molten-Cheese/releases/latest"
)

Installer_Name = "Molten-Cheese-Installer.exe"

# Check website's backend settings
class Settings(BaseModel):

    ## Prevents accidental changes
    model_config = ConfigDict(frozen=True)

    ## Allow/Disallow downloads    
    downloads_enabled: bool = False

    allowed_origins: list[str] = [
        "http://localhost:5173",
        "http://127.0.0.1:5173"
    ]

# Error & Success configs
success_cache_seconds = 300
error_cache_seconds = 30
error_github_timeout = 10
error_max_release_size = 1_000_000