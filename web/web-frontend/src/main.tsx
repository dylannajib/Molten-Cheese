import { createRoot } from "react-dom/client"
import App from "./application"
import "./appearance.css"

// Outcome: Startup fails clearly if the page has no React mount point

const container = document.getElementById("root")

if (container === null) {
    throw new Error("Missing React root element")
}


// Outcome: React displays application and manages subsequent state-driven changes

createRoot(container).render(<App />)