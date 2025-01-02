import { useState, useEffect, useCallback } from "react";
import reactLogo from "./assets/react.svg";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

function App() {

  async function gen(p) {
    // Generate text
    const response = await invoke('generate_text', { 
      prompt: p
    })
    console.log(response)
  }

  // WebSocket setup
  useEffect(() => {
// Generate text
    gen("hello world in python is what");
  }, []);


  return (
    <main className="container">
      <h1>Welcome to Tauri + React</h1>
      <div className="">
        <button onClick={() => gen("what's your favorite color")}>Click me</button>
      </div>
    </main>
  );
}

export default App;