import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");
  const [socket, setSocket] = useState(null);
  const [isConnected, setIsConnected] = useState(false);

  // WebSocket setup
  useEffect(() => {
    const ws = new WebSocket('ws://localhost:8008');

    ws.onopen = () => {
      console.log('Connected to WebSocket');
      setIsConnected(true);
    };

    ws.onmessage = (event) => {
      console.log('Received:', event.data);
      setGreetMsg(event.data);
    };

    ws.onclose = () => {
      console.log('Disconnected from WebSocket');
      setIsConnected(false);
    };

    setSocket(ws);

    // Cleanup on unmount
    return () => {
      if (ws) {
        ws.close();
      }
    };
  }, []);

  // Send message function
  const sendMessage = useCallback((message) => {
    if (socket && socket.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify(message));
    }
  }, [socket]);

  async function greet() {
    // Send via WebSocket instead of invoke
    if (isConnected) {
      sendMessage({ type: 'greet', name });
    } else {
      setGreetMsg(await invoke("greet", { name }));
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="connection-status">
        WebSocket: {isConnected ? '🟢 Connected' : '🔴 Disconnected'}
      </div>
      {/* ...existing code... */}
    </main>
  );
}

export default App;