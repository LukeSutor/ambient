"use client";
import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { Label } from "@/components/ui/label"

export default function Dev() {
  // State for SQL execution
  const [sqlQuery, setSqlQuery] = useState<string>("SELECT * FROM documents LIMIT 5;");
  const [sqlParams, setSqlParams] = useState<string>("[]"); // Store params as JSON string
  const [sqlResult, setSqlResult] = useState<string | null>(null);
  const [sqlError, setSqlError] = useState<string | null>(null);

  // State for capture scheduler
  const [isSchedulerRunning, setIsSchedulerRunning] = useState<boolean>(false);
  const [schedulerLoading, setSchedulerLoading] = useState<boolean>(false);

  // Function to start capture scheduler
  const handleStartScheduler = async () => {
    setSchedulerLoading(true);
    try {
      await invoke("start_capture_scheduler");
      setIsSchedulerRunning(true);
      console.log("Capture scheduler started");
    } catch (error: any) {
      console.error("Error starting scheduler:", error);
    } finally {
      setSchedulerLoading(false);
    }
  };

  // Function to stop capture scheduler
  const handleStopScheduler = async () => {
    setSchedulerLoading(true);
    try {
      await invoke("stop_capture_scheduler");
      setIsSchedulerRunning(false);
      console.log("Capture scheduler stopped");
    } catch (error: any) {
      console.error("Error stopping scheduler:", error);
    } finally {
      setSchedulerLoading(false);
    }
  };

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

  // --- Screen Text by Application ---
  const [screenTextData, setScreenTextData] = useState<string | null>(null);
  const [screenTextError, setScreenTextError] = useState<string | null>(null);
  const [screenTextLoading, setScreenTextLoading] = useState<boolean>(false);

  // --- Evaluation Data Capture ---
  const [evalCaptureLoading, setEvalCaptureLoading] = useState<boolean>(false);
  const [evalCaptureResult, setEvalCaptureResult] = useState<string | null>(null);
  const [evalCaptureError, setEvalCaptureError] = useState<string | null>(null);

  // --- Screen Selection ---
  const [screenSelectionResult, setScreenSelectionResult] = useState<any>(null);
  const [screenSelectionLoading, setScreenSelectionLoading] = useState<boolean>(false);

  const fetchScreenText = async () => {
    setScreenTextLoading(true);
    setScreenTextError(null);
    try {
      const data = await invoke<string>("get_screen_text_formatted");
      setScreenTextData(data);
    } catch (err: any) {
      setScreenTextError(typeof err === "string" ? err : JSON.stringify(err));
      setScreenTextData(null);
    } finally {
      setScreenTextLoading(false);
    }
  };

  const captureEvalData = async () => {
    setEvalCaptureLoading(true);
    setEvalCaptureError(null);
    setEvalCaptureResult(null);
    try {
      const result = await invoke<string>("capture_eval_data");
      setEvalCaptureResult(result);
      console.log("Evaluation data captured:", result);
    } catch (err: any) {
      setEvalCaptureError(typeof err === "string" ? err : JSON.stringify(err));
      console.error("Error capturing eval data:", err);
    } finally {
      setEvalCaptureLoading(false);
    }
  };

  // Screen Selection Functions
  const openScreenSelector = async () => {
    setScreenSelectionLoading(true);
    try {
      await invoke('open_screen_selector');
    } catch (error: any) {
      console.error('Failed to open screen selector:', error);
      setScreenSelectionLoading(false);
    }
  };

  // Listen for screen selection results
  useEffect(() => {
    const handleScreenSelection = (event: CustomEvent) => {
      const result = event.detail;
      console.log('Screen selection result received:', result);
      setScreenSelectionResult(result);
      setScreenSelectionLoading(false);
    };

    window.addEventListener('screen-selection-complete', handleScreenSelection as EventListener);
    
    return () => {
      window.removeEventListener('screen-selection-complete', handleScreenSelection as EventListener);
    };
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-center p-4 space-y-6">
      {/* Capture Scheduler Controls */}
      <div className="flex gap-4 justify-center">
        <Button 
          onClick={handleStartScheduler} 
          disabled={schedulerLoading || isSchedulerRunning}
          variant={isSchedulerRunning ? "secondary" : "default"}
        >
          {schedulerLoading && !isSchedulerRunning ? "Starting..." : "Start Capture Scheduler"}
        </Button>
        <Button 
          onClick={handleStopScheduler} 
          disabled={schedulerLoading || !isSchedulerRunning}
          variant={isSchedulerRunning ? "destructive" : "secondary"}
        >
          {schedulerLoading && isSchedulerRunning ? "Stopping..." : "Stop Capture Scheduler"}
        </Button>
      </div>

      {/* Status indicator */}
      <div className="text-sm text-center">
        Status: <span className={isSchedulerRunning ? "text-green-600 font-semibold" : "text-red-600 font-semibold"}>
          {isSchedulerRunning ? "Running" : "Stopped"}
        </span>
      </div>

      {/* Screen Text Button */}
      <div className="flex justify-center">
        <Button onClick={fetchScreenText} disabled={screenTextLoading}>
          {screenTextLoading ? "Loading..." : "Get Screen Text (Formatted)"}
        </Button>
      </div>

      {/* Screen Selection Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-blue-50">
        <h2 className="text-lg font-semibold">Screen Selection Tool</h2>
        <p className="text-sm text-gray-600">
          Click to open a fullscreen overlay where you can select any area of your screen to extract text from that specific region.
        </p>
        <Button 
          onClick={openScreenSelector} 
          disabled={screenSelectionLoading}
          variant="default"
        >
          {screenSelectionLoading ? "Select an area..." : "📱 Select Screen Area"}
        </Button>
        
        {screenSelectionResult && (
          <div className="mt-4 space-y-2">
            <h3 className="text-md font-semibold">Selection Result:</h3>
            <div className="p-2 bg-gray-100 rounded text-sm">
              <strong>Bounds:</strong> {screenSelectionResult.bounds.width}x{screenSelectionResult.bounds.height} at ({screenSelectionResult.bounds.x}, {screenSelectionResult.bounds.y})
            </div>
            <div className="p-2 bg-white border rounded text-sm max-h-64 overflow-y-auto">
              <strong>Extracted Text:</strong>
              <pre className="whitespace-pre-wrap mt-2">{screenSelectionResult.text_content || "No text found in selected area"}</pre>
            </div>
          </div>
        )}
      </div>

      {/* Evaluation Data Capture Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-orange-50">
        <h2 className="text-lg font-semibold">Evaluation Data Capture</h2>
        <p className="text-sm text-gray-600">
          Click this button when you see incorrect task detection to save the current state for evaluation. 
          Requires at least 2 screen captures and active tasks.
        </p>
        <Button 
          onClick={captureEvalData} 
          disabled={evalCaptureLoading}
          variant="outline"
        >
          {evalCaptureLoading ? "Capturing..." : "Capture Eval Data"}
        </Button>
        {evalCaptureResult && (
          <div className="mt-2 p-2 bg-green-100 border border-green-300 rounded text-sm">
            <strong>Success:</strong> {evalCaptureResult}
          </div>
        )}
        {evalCaptureError && (
          <div className="mt-2 p-2 bg-red-100 border border-red-300 rounded text-sm">
            <strong>Error:</strong> {evalCaptureError}
          </div>
        )}
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

      {/* --- Screen Text by Application Section --- */}
      <div className="w-full max-w-4xl mt-4 p-4 border rounded-md bg-yellow-50">
        <h2 className="text-lg font-semibold mb-2">Screen Text by Application (Formatted)</h2>
        {screenTextData && (
          <div className="mt-2 prose prose-sm max-w-none max-h-96 overflow-y-auto bg-white p-4 rounded border">
            <pre className="whitespace-pre-wrap text-sm">{screenTextData}</pre>
          </div>
        )}
        {screenTextError && (
          <div className="mt-2 text-red-700 font-mono">Error: {screenTextError}</div>
        )}
        {!screenTextData && !screenTextError && !screenTextLoading && (
          <div className="mt-2 text-gray-500">Click "Get Screen Text (Formatted)" button to fetch clean, organized application text data.</div>
        )}
        {screenTextLoading && (
          <div className="mt-2 text-blue-600">Loading screen text data (this may take a moment)...</div>
        )}
      </div>
    </div>
  );
}
