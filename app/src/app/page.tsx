"use client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event'; // Import listen
import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from "react"; // Import useEffect, useRef
import { Button } from "@/components/ui/button"
import Link from 'next/link'

// Define the expected payload structure
interface TaskResultPayload {
  result: string;
}

export default function Home() {
  const [greeted, setGreeted] = useState<string | null>(null);
  const greet = useCallback((): void => {
    invoke<string>("greet")
      .then((s) => {
        setGreeted(s);
      })
      .catch((err: unknown) => {
        console.error(err);
      });
  }, []);

  // New function to call the sidecar command
  async function callLlamaSidecar() {
    // Replace with actual image path
    const image = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/sample_images/gmail.png";
    // Define model and mmproj paths
    const model = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/smol.gguf";
    const mmproj = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/mmproj.gguf";
    const promptKey = "SUMMARIZE_ACTION"; // Key for the desired prompt

    try {
      // Fetch the prompt using the new command
      const prompt = await invoke<string>("get_prompt_command", { key: promptKey });
      console.log(`Fetched prompt for key '${promptKey}': ${prompt}`);

      console.log(`Calling sidecar with image: ${image}, prompt: ${prompt}`);
      const result = await invoke("call_llama_sidecar", { model, mmproj, image, prompt });
      console.log("Sidecar response:", result);
      // Handle the successful response string (result)
    } catch (error) {
      console.error("Error calling sidecar or fetching prompt:", error);
      // Handle the error string (error)
    }
  }

  async function takeScreenshot() {
    try {
      const screenshotPath = await invoke<string>("take_screenshot");
      console.log("Screenshot saved at:", screenshotPath);
      // Handle the successful screenshot path (screenshotPath)
    } catch (error) {
      console.error("Error taking screenshot:", error);
      // Handle the error string (error)
    }
  }

  // Function to start the scheduler
  async function startScheduler() {
    try {
      // Optionally pass an interval in minutes: await invoke("start_scheduler", { interval: 5 });
      await invoke("start_scheduler");
      console.log("Scheduler started successfully.");
      // You could update UI state here, e.g., disable start button, enable stop button
    } catch (error) {
      console.error("Error starting scheduler:", error);
      // Handle the error
    }
  }

  // Function to stop the scheduler
  async function stopScheduler() {
    try {
      await invoke("stop_scheduler");
      console.log("Scheduler stopped successfully.");
      // You could update UI state here, e.g., enable start button, disable stop button
    } catch (error) {
      console.error("Error stopping scheduler:", error);
      // Handle the error (e.g., scheduler wasn't running)
    }
  }

  const [taskResults, setTaskResults] = useState<string[]>([]); // State for task results
  const resultsEndRef = useRef<HTMLDivElement>(null); // Ref for scrolling

  // Effect to listen for task results
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    async function setupListener() {
      try {
        unlisten = await listen<TaskResultPayload>('task-completed', (event) => {
          console.log("Received task result:", event.payload.result);
          setTaskResults((prevResults) => [...prevResults, event.payload.result]);
        });
        console.log("Event listener for 'task-completed' set up.");
      } catch (error) {
        console.error("Failed to set up event listener:", error);
      }
    }

    setupListener();

    // Cleanup listener on component unmount
    return () => {
      if (unlisten) {
        unlisten();
        console.log("Event listener for 'task-completed' cleaned up.");
      }
    };
  }, []); // Empty dependency array ensures this runs only once on mount

  // Effect to scroll to the bottom of the results box when new results arrive
  useEffect(() => {
    resultsEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [taskResults]);

  return (
    <div className="relative flex flex-col items-center justify-center p-4">
      <main className="flex flex-col gap-4 items-center sm:items-start w-full max-w-md">
        {/* Results Box */}
        <div className="w-full mt-4 p-4 border rounded-md h-64 overflow-y-auto bg-gray-50">
          <h2 className="text-lg font-semibold mb-2">Task Results:</h2>
          {taskResults.length === 0 ? (
            <p className="text-gray-500">No results yet. Start the scheduler.</p>
          ) : (
            taskResults.map((result, index) => (
              <pre key={index} className="whitespace-pre-wrap text-sm p-2 mb-2 border-b last:border-b-0">
                {result}
              </pre>
            ))
          )}
          {/* Invisible element to scroll to */}
          <div ref={resultsEndRef} />
        </div>
      </main>
    </div>
  );
}
