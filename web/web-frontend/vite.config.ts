import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";   // Enable React and connect development server to API

// Outcome: React runs locally and /api reaches separate backend without mixing source files

export default defineConfig({               // Export frontend config
    plugins: [react()],

    server: {                               // Start local server settings
        host: "127.0.0.1",
        port: 5173,
        strictPort: true,                   // Fail if port 5173 is occupied

        proxy: {                            // Forward API requests w/o moving backend code into the frontend
            "/api": {
                target: "http://127.0.0.1:8000",
                changeOrigin: false
            }
        }
    }
})