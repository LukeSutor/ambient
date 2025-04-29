"use client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event'; // Import listen
import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea" // Import Textarea
import { Label } from "@/components/ui/label" // Import Label
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
  async function callVLMSidecar() {
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
      const result = await invoke("get_vlm_response", { model, mmproj, image, prompt });
      console.log("Sidecar response:", result);
      // Handle the successful response string (result)
    } catch (error) {
      console.error("Error calling sidecar or fetching prompt:", error);
      // Handle the error string (error)
    }
  }

  async function callEmbeddingSidecar() {
    const model = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/smol.gguf";
    const prompt = "Hello world!";

    try {
      console.log(`Calling embedding sidecar with model: ${model}, prompt: ${prompt}`);
      const embedding = await invoke<string>("get_embedding", { model, prompt });
      console.log("Embedding response:", embedding);
      // Handle the successful embedding response (embedding)
    } catch (error) {
      console.error("Error calling embedding sidecar:", error);
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

  // State for SQL execution
  const [sqlQuery, setSqlQuery] = useState<string>("SELECT * FROM documents LIMIT 5;");
  const [sqlParams, setSqlParams] = useState<string>("[]"); // Store params as JSON string
  const [sqlResult, setSqlResult] = useState<string | null>(null);
  const [sqlError, setSqlError] = useState<string | null>(null);

  // Function to execute SQL
  const handleExecuteSql = async () => {
    setSqlResult(null); // Clear previous result
    setSqlError(null); // Clear previous error

    let parsedParams: any[] | null = null;
    try {
      // Only parse if params string is not empty and not just whitespace
      if (sqlParams.trim()) {
        parsedParams = JSON.parse(sqlParams);
        if (!Array.isArray(parsedParams)) {
          throw new Error("Parameters must be a valid JSON array.");
        }
      }
    } catch (e: any) {
      setSqlError(`Invalid JSON in parameters: ${e.message}`);
      return;
    }

    try {
      console.log(`Executing SQL: ${sqlQuery} with params:`, parsedParams);
      const result = await invoke("execute_sql", {
        sql: sqlQuery,
        params: parsedParams, // Pass null if parsing resulted in null
      });
      console.log("SQL Result:", result);
      setSqlResult(JSON.stringify(result, null, 2)); // Pretty print JSON result
    } catch (error: any) {
      console.error("Error executing SQL:", error);
      setSqlError(typeof error === 'string' ? error : JSON.stringify(error));
    }
  };

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
    <div className="relative flex flex-col items-center justify-center p-4 space-y-6">
      {/* Existing Buttons Section */}
      <div className="flex flex-wrap gap-2 justify-center">
        <Button variant="outline" onClick={callVLMSidecar}>Call VLM Sidecar</Button>
        <Button variant="outline" onClick={callEmbeddingSidecar}>Call Embedding Sidecar</Button>
        <Button onClick={takeScreenshot}>Take Screenshot</Button>
        <Button onClick={startScheduler}>Start Scheduler</Button>
        <Button onClick={stopScheduler} variant="destructive">Stop Scheduler</Button>
      </div>

      {/* SQL Execution Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4">
        <h2 className="text-lg font-semibold">Execute SQL Query</h2>
        <div className="space-y-2">
          <Label htmlFor="sql-query">SQL Query</Label>
          <Textarea
            id="sql-query"
            value={sqlQuery}
            onChange={(e) => setSqlQuery(e.target.value)}
            placeholder="Enter SQL query (e.g., SELECT * FROM documents)"
            rows={4}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="sql-params">Parameters (JSON Array)</Label>
          <Textarea
            id="sql-params"
            value={sqlParams}
            onChange={(e) => setSqlParams(e.target.value)}
            placeholder='Enter parameters as JSON array (e.g., ["value1", 123]) or leave empty'
            rows={2}
          />
        </div>
        <Button onClick={handleExecuteSql}>Execute SQL</Button>
        {(sqlResult || sqlError) && (
          <div className="mt-4">
            <h3 className="text-md font-semibold">Result:</h3>
            <pre className="mt-2 p-2 border rounded bg-gray-50 text-sm overflow-x-auto">
              {sqlError ? `Error: ${sqlError}` : sqlResult}
            </pre>
          </div>
        )}
      </div>

      {/* Results Box */}
      <div className="w-full max-w-2xl mt-4 p-4 border rounded-md h-64 overflow-y-auto bg-gray-50">
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
    </div>
  );
}
