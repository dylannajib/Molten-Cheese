import { useCallback, useEffect, useRef, useState } from "react";
import { fetchLatestRelease, type ReleaseInfo } from "./release";

// Outcome: Connection failure stays separate from the backend's coming-soon status

type ReleaseState = {
    phase: "loading" | "ready" | "error";                               // Track request's phase
    release: ReleaseInfo | null;
    error: string | null
};

const REQUEST_TIMEOUT_MS = 15_000;


// Outcome: ONe shared hook owns release state, initial loading and refresh requests

export function useRelease() {
    const [state, setState] = useState<ReleaseState>({
        phase: "loading",
        release: null,
        error: null
    });

    const activateRequest = useRef<AbortController | null> (null);

    // Sub-Outcome: Each refresh returns fresh validated data or null, keeping prior metadata available

    const refresh = useCallback(async (): Promise<ReleaseInfo | null> => {
        const previousRequest = activateRequest.current;
        const controller = new AbortController();
        
        // Sub-Outcome: Old results cannot overwrite newer refreshes

        activateRequest.current = controller;
        previousRequest?.abort();

        setState((previous) => ({
            ...previous,
            phase: "loading",
            error: null
        }));

        // Sub-Outcome: Stalled requests become a retryable failure after 15 sec

        const timeout = window.setTimeout(() => {controller.abort()}, REQUEST_TIMEOUT_MS);

        try {
            const release = await fetchLatestRelease(controller.signal)

            if (activateRequest.current !== controller) {
                return null
            }

            setState({
                phase: "ready",
                release,
                error: null
            });

            return release
        }
        catch {
            if (activateRequest.current === controller) {
                setState((previous) => ({
                    ...previous,
                    phase: "error",
                    error: "Cannot reach release info. Please retry"
                }));
            }

            return null;
        }
        finally {
            window.clearTimeout(timeout);

            if (activateRequest.current === controller) {
                activateRequest.current = null
            }
        }
    }, []);

    // Sub-Outcome: Opening the page loads metadata, while leaving cancels outstanding work

    useEffect(() => {
        
        void refresh();

        return () => {
            const controller = activateRequest.current;
            activateRequest.current = null;
            controller?.abort();
        };
    }, [refresh])

    return {
        ...state,
        refresh
    }
}