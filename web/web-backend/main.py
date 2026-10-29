import asyncio
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from config import Settings
from releases import get_cached_release
from schemas import ReleaseInfo

# Prep shared settings, storage and lock when server starts
@asynccontextmanager
async def lifespan(app: FastAPI):
    app.state.settings = Settings()

    app.state.release_cache = {
        "release": None,
        "expires_at": 0.0
    }

    app.state.release_lock = asyncio.Lock()

    yield

# Confirms backend is running
async def get_health() -> dict[str, str]:
    return {"status":"ok"}

# Displays coming soon info or latest installer details
async def get_latest_release(request: Request):
    settings = request.app.state.settings

    if not settings.downloads_enabled:
        return ReleaseInfo(status="coming_soon")

    ## Lets only one request refresh the shared cache at a time
    async with request.app.state.release_lock:
        release = await get_cached_release(request.app.state.release_cache)

    if release.status == "unavailable":
        return JSONResponse(
            status_code=503, ### 503 = Web temporarily unavailable to handle request
            content=release.model_dump(),
            headers={"Retry-After":"30"}
        )

    return release

# Browser protection and caching instructions
async def add_response_headers(request: Request, calls_next):
    response = await calls_next(request)

    response.headers["X-Content-Type-Options"] = "nosniff"
    response.headers["Referrer-Policy"] = "no-referrer"
    response.headers["Cache-Control"] = "no-store"

    return response

# Assembles FastAPI application, middleware and API routes
def create_app() -> FastAPI:
    application = FastAPI(
        title="Molten Cheese Website API",
        lifespan=lifespan,
        docs_url=None,
        redoc_url=None,
        openapi_url=None
    )

    settings = Settings()

    application.add_middleware(
        CORSMiddleware,
        allow_origins=list(settings.allowed_origins),
        allow_credentials=False,
        allow_methods=["GET"],
        allow_headers=[]
    )

    application.middleware("http")(add_response_headers)

    application.add_api_route(
        "/api/health",
        get_health,
        methods=["GET"]
    )

    application.add_api_route(
        "/api/releases/latest",
        get_latest_release,
        methods=["GET"],
        response_model=ReleaseInfo
    )

    return application

app = create_app()