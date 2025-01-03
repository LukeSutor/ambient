import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {

  const shutdownSidecarAction = async () => {
    console.log("shutdown server");
    try {
      await invoke("shutdown_server");
      return;
    } catch (err) {
      console.error(`[ui] Failed to shutdown server. ${err}`);
    }
  }

  const startSidecarAction = async () => {
    console.log("start server");
    try {
      await invoke("start_server");
      return;
    } catch (err) {
      console.error(`[ui] Failed to start server. ${err}`);
    }
  }

  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="">
        <button onClick={startSidecarAction}>Connect</button>
        <button onClick={shutdownSidecarAction}>Disconnect</button>
      </div>
    </main>
  );
}

export default App;