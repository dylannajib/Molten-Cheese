// Outcome: All frontend release code shares the same expected field names & types

export type ReleaseInfo = {                                     // Describes the release

    status: "coming_soon" | "available" | "unavailable";
    version: string | null;
    file_size: number | null;
    release_notes: string;
    download_url: string | null;
};


// Outcome: Rejects download destinations outside the Molten Cheese Github repo

const ALLOWED_DOWNLOAD_URL =
      /^https:\/\/github\.com\/dylannajib\/Molten-Cheese\/releases\/download\/[^/?#\s]+\/Molten-Cheese-Installer\.exe$/;


// Outcome: Invalid response data cannot reach normal rendering & download path

export function parseReleaseInfo(value: unknown):               // Checks unknown data & return a valid release
ReleaseInfo {
    if (
        value === null ||
        typeof value !== "object" ||
        Array.isArray(value)                                    // Rejects lists of values
    ) {
        throw new Error("Invalid release response");
    }

    const data = value as Record<string, unknown>;              // Let TypeScript inspect named fields
    const status = data.status;
    const version = data.version;
    const size = data.file_size;
    const notes = data.release_notes;
    const url = data.download_url;

    if (                                                        // Accepts only the 3 release statuses defined by backend
        status !== "coming_soon" &&
        status !== "available" &&
        status !== "unavailable"
    ) {
        throw new Error("Invalid release status");
    }

    if (                                                        // Match backend's optional version and its length limits
        version !== null && (
            typeof version !== "string" ||
            version.length < 1 ||
            version.length > 100
        )
    ) {
        throw new Error("Invalid release version")
    }

    if (                                                        // Installer sizes must be positive whole numbers if provided
        size !== null &&
        (
            typeof size !== "number" ||
            !Number.isSafeInteger(size) ||
            size <= 0
        )
    ) {
        throw new Error("Invalid installer size")
    }

    if (                                                        // Keep release notes within backend's defined limit
        typeof notes !== "string" ||
        notes.length > 100_000
    ) {
        throw new Error("Invalid release notes")
    }

    if (
        url !== null &&
        typeof url !== "string"
    ) {
        throw new Error("Invalid download URL")
    }

    if (status === "available") {                               // Available releases require complete & trusted download info
        if (
            typeof version !== "string" ||
            version.trim().length === 0 ||
            typeof size !== "number" ||
            typeof url !== "string" ||
            !ALLOWED_DOWNLOAD_URL.test(url)
        ) {
            throw new Error("Invalid installer details")
        }
    }   else if (url !== null) {
        throw new Error("Inactive release contains a download URL")
    }

    return {
        status,
        version: version as string | null,
        file_size: size as number | null,
        release_notes: notes,
        download_url: url as string | null
    };
}


// Outcome: Only validated JSON responses (incl. documented unavailable result) reach callers

export async function fetchLatestRelease(signal: AbortSignal):
Promise<ReleaseInfo> {
    const response = await fetch ("/api/releases/latest", {
        method: "GET",
        headers: {Accept: "application/json"},
        credentials: "omit",
        cache: "no-store",
        signal,
    });

    const contentType = response.headers.get("content-type") ?? "";

    if (!contentType.includes("application/json")) {
        throw new Error("Unexpected server response")
    }

    const release = parseReleaseInfo(await response.json());

    const expectedUnavailable = 
        response.status === 503 &&
        release.status === "unavailable";
    
    if (!response.ok && !expectedUnavailable) {
        throw new Error("Release request failed");
    }

    return release;
}


// Outcome: Missing sizes show X mb whilst real byte counts show decimal mb

export function formatFileSize(bytes: number | null):
string {
    return bytes === null
        ? "X mb"                                                 // Placeholder when missing
        : `${(bytes / 1_000_000).toFixed(1)} mb`;                // Otherwise show decimal mb to 1 dec. place
}
