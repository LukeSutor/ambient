"use client";
import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";
import { Button } from "@/components/ui/button"
import { Textarea } from "@/components/ui/textarea"
import { Label } from "@/components/ui/label"

export default function Dev() {
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

  // --- Screen Text by Application ---
  const [screenTextData, setScreenTextData] = useState<string | null>(null);
  const [screenTextError, setScreenTextError] = useState<string | null>(null);
  const [screenTextLoading, setScreenTextLoading] = useState<boolean>(false);

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

  return (
    <div className="relative flex flex-col items-center justify-center p-4 space-y-6">
      {/* Screen Text Button */}
      <div className="flex justify-center">
        <Button onClick={fetchScreenText} disabled={screenTextLoading}>
          {screenTextLoading ? "Loading..." : "Get Screen Text (Formatted)"}
        </Button>
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
