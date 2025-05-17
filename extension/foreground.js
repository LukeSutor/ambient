// This script gets injected into any opened page
// whose URL matches the pattern defined in the manifest
// (see "content_script" key).
// Several foreground scripts can be declared
// and injected into the same or different pages.

console.log("This prints to the console of the page (injected only if the page url matched)")

// Minimal WebSocket client that tries a range of ports to connect to the Tauri server
let ws = null;
let connectedPort = null;
const PORT_RANGE = Array.from({length: 11}, (_, i) => 3010 + i);

async function connectToTauriWebSocket() {
    for (const port of PORT_RANGE) {
        try {
            const socket = new WebSocket(`ws://127.0.0.1:${port}/ws`);
            await new Promise((resolve, reject) => {
                socket.onopen = () => resolve();
                socket.onerror = () => reject();
                setTimeout(() => reject(), 500); // timeout
            });
            ws = socket;
            connectedPort = port;
            console.log("[extension] Connected to Tauri WebSocket on port", port);

            ws.onmessage = (event) => {
                // Handle messages from Tauri here
                console.log("[extension] Message from Tauri:", event.data);
            };
            break;
        } catch {
            // Try next port
        }
    }
    if (!ws) {
        console.warn("[extension] Could not connect to Tauri WebSocket server on any port");
    }
}

// Call this to send a message to Tauri
function sendMessageToTauri(obj) {
    if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(JSON.stringify(obj));
    } else {
        console.warn("[extension] WebSocket not connected");
    }
}

// Try to connect on load
connectToTauriWebSocket();
