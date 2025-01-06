import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {
  const [input, setInput] = useState("");
  const [prompt, setPrompt] = useState("");
  const [includeImage, setIncludeImage] = useState(false);

  const shutdownSidecarAction = async () => {
    console.log("shutdown server");
    try {
      const result = await invoke("shutdown_sidecar");
      console.log("Shutdown result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to shutdown server. ${err}`);
    }
  }

  const startSidecarAction = async () => {
    console.log("start server");
    try {
      const result = await invoke("start_sidecar");
      console.log("Start result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to start server. ${err}`);
    }
  }

  const inferAction = async () => {
    console.log("making inference request", prompt, includeImage);
    try {
      const result = await invoke("handle_request", { prompt, includeImage });
      console.log("Inference result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to make inference request. ${err}`);
    }
  }

  const takeScreenshotAction = async () => {
    console.log("taking screenshot");
    try {
      const result = await invoke("take_screenshot");
      console.log("Screenshot result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to take screenshot. ${err}`);
    }
  }

  const writeSidecarAction = async () => {
    console.log("writing to sidecar: ", input);
    try {
      const result = await invoke("write_to_sidecar", { message: input });
      console.log("Sidecar result:", result);
      return;
    } catch (err) {
      console.error(`[ui] Failed to write to sidecar. ${err}`);
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="">
        <button onClick={startSidecarAction}>Connect</button>
        <button onClick={shutdownSidecarAction}>Disconnect</button>
        <button onClick={takeScreenshotAction}>Take Screenshot</button>
      </div>
      <div className="">
        <input
          type="text"
          placeholder="Enter sidecar input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
        />
        <button onClick={writeSidecarAction}>Write to Sidecar</button>
      </div>
      <div className="">
        <input
          type="text"
          placeholder="Enter prompt"
          value={prompt}
          onChange={(e) => setPrompt(e.target.value)}
        />
        <label>
          <input
            type="checkbox"
            checked={includeImage}
            onChange={(e) => setIncludeImage(e.target.checked)}
          />
          Include Screenshot
        </label>
        <button onClick={inferAction}>Submit</button>
      </div>
    </main>
  );
}

export default App;