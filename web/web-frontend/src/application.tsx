import { useEffect, useRef, useState } from "react";
import { formatFileSize, type ReleaseInfo } from "./release";
import { useRelease } from "./useRelease";
import logo from "../image-assets/molten-cheese-logo.png";
import mascot from "../image-assets/molten-cheese-mascot.png";
import windowsClassic from "../image-assets/windows-classic.svg";
import githubMark from "../image-assets/github-mark.png";

const COMING_SOON_MS = 2_500;

// Outcome: Keep the website's first dated note separate from downloadable app metadata

const INITIAL_RELEASE_NOTE = "v0.0.0 - Hello world!";

// Outcome: Windows logo on download button

function WindowsMark() {
    return (
        <img className="windows-mark" src={windowsClassic} alt="" aria-hidden="true" />
    )
}


// Outcome: Button receives loading infromation & shared release-check operation

type DownloadButtonProps = {
    release: ReleaseInfo | null;
    checking: boolean
    onCheck: () => Promise<ReleaseInfo | null>;
}


// Outcome: Button rechecks backend and temporarily change its label

function DownloadButton({release, checking, onCheck}: DownloadButtonProps) {
    const [busy, setBusy] = useState(false);
    const [notice, setNotice] = useState<
        "coming_soon" | "unavailable" | null
    >(null);

    const timer = useRef<number | null>(null);                              // Remembers temporary message timer
    const mounted = useRef(true);

    const clickLock = useRef(false);

    // Sub-Outcome: Leaving the page cancels the button's temporary-label timer
    
    useEffect(() => {
        mounted.current=true
        return () => {
            mounted.current=false
                
            if (timer.current !== null) {
                window.clearTimeout(timer.current)
            }
        }
    }, []);

    // Sub-Outcome: Click either confirms coming soon, temporary unavailability or starts a real download

    async function handleDownload() {
        if (clickLock.current || checking || notice !== null) {
            return
        }

        clickLock.current=true
        setBusy(true)

        try {
            const release = await onCheck();

            if (!mounted.current) {
                return
            }

            if (release?.status === "available" && release.download_url) {
                window.location.assign(release.download_url)
                return
            }

            setNotice(release?.status === "coming_soon" ? "coming_soon" : "unavailable")

            // Sub-Sub-Outcome: Label resets after 2.5 sec

            timer.current = window.setTimeout(() => {
                setNotice(null)
                timer.current=null
            }, COMING_SOON_MS);
        }
        finally {
            clickLock.current=false

            if (mounted.current) {
                setBusy(false)
            }
        }
    }
    
    let label = "Download for Windows";

    if (notice === "coming_soon") {
        label = "Coming soon!"
    }
    else if (notice === "unavailable") {
        label = "Temporarily unavailable"
    }
    else if (busy || checking) {
        label = "Checking..."
    }

    return (
        <button
            className="download-button"
            type="button"
            onClick={handleDownload}
            disabled={busy || checking || notice !== null}
            aria-busy={busy || checking}
            aria-label={`${label}, Supported: Windows 11`}
        >
            <WindowsMark />
            <span className="download-copy">
                <span className="download-label" aria-live="polite">
                    {label === "Download for Windows" ? (
                        <>
                            <span className="download-prefix">Download for</span>{" "}
                            <span className="download-platform">Windows</span>
                        </>
                    ) : <span className="download-notice">{label}</span>}
                </span>

                <span className="download-subtext">
                    Supported: Windows 11
                </span>
            </span>
            {/* Sub-Outcome: Fixed metadata stays on the right while the main label changes */}
            <span className="download-side">
                <span className="download-metadata">
                    <span className="download-version">{release?.version ?? "v0.0.1-a"}</span>
                    <span className="download-size">{formatFileSize(release?.file_size ?? null)}</span>
                </span>
            </span>
        </button>
    )
}


// Outcome: Notes component receive data and connection state w/o making its own requests

type ReleaseDetailsProps = {
    release: ReleaseInfo | null
}



// Outcome: Display safe release notes, version and size without extra requests

function ReleaseDetails({release}: ReleaseDetailsProps) {
    const notes = release?.release_notes || INITIAL_RELEASE_NOTE;

    // Sub-Outcome: A native button controls a smoothly expanding, accessible notes region

    const [expanded, setExpanded] = useState(false);

    return (
        <section className="release-details" aria-label="Release information">
            <div className="release-notes">
                <button className="release-notes-toggle" type="button"
                    aria-expanded={expanded} aria-controls="release-notes-content"
                    onClick={() => setExpanded(previous => !previous)}>
                    <span className="release-notes-arrow" aria-hidden="true" />
                    <span>Release Notes</span>
                </button>
                <div id="release-notes-content"
                    className={`release-notes-content${expanded ? " is-open" : ""}`}
                    aria-hidden={!expanded} inert={!expanded}>
                    <div className="release-notes-clip">
                        <div className="release-notes-body">
                {/* Sub-Outcome: React displays notes as text instead of executable HTML */}
                            <p>{notes}</p>
                            {notes === INITIAL_RELEASE_NOTE && (
                                <time className="release-date" dateTime="2026-10-30">30/10/2026</time>
                            )}
                        </div>
                    </div>
                </div>
            </div>
        </section>
    );
}


// Outcome: Shared release state keeps the page, download and retry controls in sync

export default function App() {
    const {phase, release, refresh} = useRelease();
    const checking = phase === "loading";

    return (
        <main className="landing">
            <div className="page-content">
                {/* Sub-Outcome: Artwork bounds reproduce the reference layout without transparent image padding */}
                <div className="hero-stage">
                    <div className="hero-canvas">
                    <header className="brand">
                        <h1><img className="brand-logo" src={logo} alt="Molten Cheese" /></h1>
                    </header>
                    <div className="hero-layout">
                    <section className="hero-copy" aria-label="About and download">
                        <h2>Honeyfile &amp; EDR Antivirus</h2>
                        <DownloadButton release={release} checking={checking} onCheck={refresh} />
                        <a className="github-link" href="https://github.com/dylannajib"
                            target="_blank" rel="noopener noreferrer">
                            <img src={githubMark} className="github-mark" alt="" aria-hidden="true" />
                            <span>GitHub</span>
                        </a>
                    </section>
                    <div className="mascot-frame">
                        <img className="mascot" src={mascot} alt="Molten Cheese mascot holding a mousetrap" />
                    </div>
                    </div>
                    </div>
                </div>
                <section className="additional-content" aria-label="Description and release notes">
                    <p className="memorial"><em>In memory of the Cheese Worm, 2001</em></p>
                    <div className="description">
                        <p>Dairyfile: Safe honeyfiles ("decoys") spread across your computer to catch malware processes off-guard. Once opened, Dairyfiles will suspend the malware process and notify both you and ChEDR.</p>
                        <p>ChEDR: (Chelating Endpoint Detection and Response) Monitors your computer continuously for any cyber threats. Unlike traditional EDRs, ChEDR will not interrupt legitimate processes of the infected host. ChEDR will besiege it to suspend any outgoing malware processes until it could find, isolate and quarantine the malware.</p>
                        <p>Afterwards, you can safely remove or disinfect the malware for careful study</p>
                    </div>
                    <ReleaseDetails release={release} />
                </section>
            </div>
        </main>
    );
}
