import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [prompt, setPrompt] = useState("");
  const [imagePath, setImagePath] = useState("");

  const shutdownSidecarAction = async () => {
    console.log("shutdown server");
    try {
      const result = await invoke("shutdown_server");
      console.log("Shutdown result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to shutdown server. ${err}`);
    }
  }

  const startSidecarAction = async () => {
    console.log("start server");
    try {
      const result = await invoke("start_server");
      console.log("Start result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to start server. ${err}`);
    }
  }

  const inferAction = async () => {
    console.log("making inference request", prompt, imagePath);
    try {
      const result = await invoke("infer", { prompt, imagePath });
      console.log("Inference result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to make inference request. ${err}`);
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="">
        <button onClick={startSidecarAction}>Connect</button>
        <button onClick={shutdownSidecarAction}>Disconnect</button>
      </div>
      <div className="">
        <input
          type="text"
          placeholder="Enter prompt"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
        />
        <input
          type="text"
          placeholder="Enter image path (optional)"
          value={imagePath}
          onChange={(e) => setImagePath(e.target.value)}
        />
        <button onClick={inferAction}>Submit</button>
      </div>
    </main>
  );
}

export default App;