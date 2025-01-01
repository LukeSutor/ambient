import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {

  async function gen() {
    // Generate text
    const response = await invoke('generate_text', { 
      prompt: 'Write a story about...'
    })
    console.log(response)
  }

  // WebSocket setup
  useEffect(() => {
// Generate text
    gen();
  }, []);


  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="">
      </div>
    </main>
  );
}

export default App;