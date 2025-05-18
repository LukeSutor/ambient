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

// --- Workflow Format Types ---
export interface Workflow {
  steps: Step[];
  name: string;
  description: string;
  version: string;
  input_schema: [];
}
export type Step =
  | NavigationStep
  | ClickStep
  | InputStep
  | KeyPressStep
  | ScrollStep;

export interface BaseStep {
  type: string;
  timestamp: number;
  tabId: number;
  url?: string;
}
export interface NavigationStep extends BaseStep {
  type: "navigation";
  url: string;
}
export interface ClickStep extends BaseStep {
  type: "click";
  url: string;
  xpath: string;
  cssSelector?: string;
  elementTag: string;
  elementText: string;
}
export interface InputStep extends BaseStep {
  type: "input";
  url: string;
  xpath: string;
  cssSelector?: string;
  elementTag: string;
  value: string;
}
export interface KeyPressStep extends BaseStep {
  type: "key_press";
  url?: string;
  key: string;
  xpath?: string;
  cssSelector?: string;
  elementTag?: string;
}
export interface ScrollStep extends BaseStep {
  type: "scroll";
  targetId: number;
  scrollX: number;
  scrollY: number;
}

import React from "react";

// --- WebSocket Event Monitor ---
const WS_PORT_RANGE = Array.from({ length: 11 }, (_, i) => 3010 + i);

function useWebSocketEventMonitor() {
  const [events, setEvents] = useState<Step[]>([]);
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    let connected = false;
    let ws: WebSocket | null = null;
    (async () => {
      for (const port of WS_PORT_RANGE) {
        try {
          ws = new WebSocket(`ws://127.0.0.1:${port}/ws`);
          await new Promise((resolve, reject) => {
            ws!.onopen = () => resolve(undefined);
            ws!.onerror = () => reject(undefined);
            setTimeout(() => reject(undefined), 500);
          });
          connected = true;
          wsRef.current = ws;
          ws.onmessage = (event) => {
            try {
              const data = JSON.parse(event.data);
              // Only add if it matches a Step type (basic check)
              if (data && typeof data.type === "string" && data.timestamp) {
                setEvents((prev) => [...prev, data]);
              }
            } catch (e) {
              // Ignore non-JSON or non-event messages
            }
          };
          break;
        } catch {
          // Try next port
        }
      }
    })();
    return () => {
      if (wsRef.current) wsRef.current.close();
    };
  }, []);
  return events;
}

export default function Dev() {
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

  const events = useWebSocketEventMonitor();

  // --- Workflows Viewer ---
  const [workflows, setWorkflows] = useState<any[]>([]);
  const [workflowsLoading, setWorkflowsLoading] = useState(false);
  const [workflowsError, setWorkflowsError] = useState<string | null>(null);

  useEffect(() => {
    setWorkflowsLoading(true);
    setWorkflowsError(null);
    invoke<any[]>("get_workflows_global", { offset: 0, limit: 10 })
      .then((result) => {
        // result is expected to be an array of workflow objects
        setWorkflows(Array.isArray(result) ? result : []);
        setWorkflowsLoading(false);
      })
      .catch((err) => {
        setWorkflowsError(typeof err === "string" ? err : JSON.stringify(err));
        setWorkflowsLoading(false);
      });
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-center p-4 space-y-6">
      {/* Existing Buttons Section */}
      <div className="flex flex-wrap gap-2 justify-center">
        <Button variant="outline" onClick={callVLMSidecar}>Call VLM Sidecar</Button>
        <Button variant="outline" onClick={callEmbeddingSidecar}>Call Embedding Sidecar</Button>
        <Button onClick={takeScreenshot}>Take Screenshot</Button>
        <Button onClick={startScheduler}>Start Scheduler</Button>
        <Button onClick={stopScheduler} variant="destructive">Stop Scheduler</Button>
        <Button asChild><Link href="/setup">Setup</Link></Button>
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
      </div>

      {/* --- Browser Event Monitor --- */}
      <div className="w-full max-w-2xl mt-4 p-4 border rounded-md h-64 overflow-y-auto bg-blue-50">
        <h2 className="text-lg font-semibold mb-2">Browser Event Monitor</h2>
        {events.length === 0 ? (
          <p className="text-gray-500">No browser events received yet.</p>
        ) : (
          events.slice(-50).map((event, idx) => (
            <div key={idx} className="mb-2 p-2 border-b last:border-b-0 bg-white rounded">
              <div className="font-mono text-xs text-gray-700">
                <b>{event.type}</b> @ {new Date(event.timestamp).toLocaleTimeString()} (tab {event.tabId})
              </div>
              <div className="text-xs text-gray-600">
                {Object.entries(event)
                  .filter(([k]) => !["type", "timestamp", "tabId"].includes(k))
                  .map(([k, v]) => (
                    <span key={k} className="mr-2">
                      <b>{k}:</b> {typeof v === "string" || typeof v === "number" ? v : JSON.stringify(v)}
                    </span>
                  ))}
              </div>
            </div>
          ))
        )}
      </div>

      {/* --- Workflows Table Viewer --- */}
      <div className="w-full max-w-2xl mt-4 p-4 border rounded-md h-64 overflow-y-auto bg-green-50">
        <h2 className="text-lg font-semibold mb-2">Saved Workflows (First 10)</h2>
        {workflowsLoading ? (
          <p className="text-gray-500">Loading workflows...</p>
        ) : workflowsError ? (
          <p className="text-red-500">Error: {workflowsError}</p>
        ) : workflows.length === 0 ? (
          <p className="text-gray-500">No workflows found.</p>
        ) : (
          workflows.map((wf, idx) => (
            <div key={wf.id ?? idx} className="mb-2 p-2 border-b last:border-b-0 bg-white rounded">
              <div className="font-mono text-xs text-gray-700">
                <b>{wf.name}</b> ({wf.url ?? "no url"})
              </div>
              <div className="text-xs text-gray-600">
                <b>Description:</b> {wf.description ?? <span className="italic text-gray-400">none</span>}
                <br />
                <b>Steps:</b> {(() => {
                  try {
                    const steps = JSON.parse(wf.steps_json ?? "[]");
                    return Array.isArray(steps) ? steps.length : 0;
                  } catch {
                    return 0;
                  }
                })()}
                <br />
                <b>Recorded:</b>{" "}
                {wf.recording_start
                  ? new Date(wf.recording_start * 1000).toLocaleString()
                  : "?"}
                {" - "}
                {wf.recording_end
                  ? new Date(wf.recording_end * 1000).toLocaleString()
                  : "?"}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
