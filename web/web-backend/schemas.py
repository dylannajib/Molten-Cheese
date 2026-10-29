from typing import Literal
from pydantic import BaseModel, Field, ConfigDict, model_validator

# Checks release information sent to the website
class ReleaseInfo(BaseModel):
    model_config = ConfigDict(
        extra="forbid",
        frozen=True
    )
    
    status: Literal[
        "coming_soon",
        "available",
        "unavailable"
    ]

    version: str | None = Field(
        default=None,
        min_length=1,
        max_length=100
    )

    file_size: int | None = Field(
        default=None,
        gt=0
    )

    release_notes: str = Field(
        default="",
        max_length=100_000
    )

    download_url: str | None = None

    ## For requiring complete download details
    @model_validator(mode="after")
    def check_download_details(self) -> "ReleaseInfo":
        if self.status == "available":
            if not self.version or self.file_size is None or not self.download_url:
                raise ValueError("Available releases require version, size and URL")
        elif self.download_url is not None:
            raise ValueError(
                "Inactive releases cannot provide a download URL"
            )

        return self