"use client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import type { SupabaseUser } from "@/types/auth";
import type { OcrResponseEvent } from "@/types/events";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

interface ScreenSelectionResult {
  bounds: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  text_content: string;
}

export default function Dev() {
  // State for SQL execution
  const [sqlQuery, setSqlQuery] = useState<string>(
    "SELECT * FROM documents LIMIT 5;",
  );
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
    } catch (error) {
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
    } catch (error) {
      console.error("Error stopping scheduler:", error);
    } finally {
      setSchedulerLoading(false);
    }
  };

  // Function to execute SQL
  const handleExecuteSql = async () => {
    setSqlResult(null); // Clear previous result
    setSqlError(null); // Clear previous error

    let parsedParams: unknown[] | null = null;
    try {
      // Only parse if params string is not empty and not just whitespace
      if (sqlParams.trim()) {
        const parsed: unknown = JSON.parse(sqlParams);
        if (Array.isArray(parsed)) {
          parsedParams = parsed as unknown[];
        } else {
          throw new Error("Parameters must be a valid JSON array.");
        }
      }
    } catch (e) {
      setSqlError(
        `Invalid JSON in parameters: ${e instanceof Error ? e.message : String(e)}`,
      );
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
    } catch (error) {
      console.error("Error executing SQL:", error);
      setSqlError(typeof error === "string" ? error : JSON.stringify(error));
    }
  };

  // --- Screen Text by Application ---
  const [screenTextData, setScreenTextData] = useState<string | null>(null);
  const [screenTextError, setScreenTextError] = useState<string | null>(null);
  const [screenTextLoading, setScreenTextLoading] = useState<boolean>(false);

  // --- Evaluation Data Capture ---
  const [evalCaptureLoading, setEvalCaptureLoading] = useState<boolean>(false);
  const [evalCaptureResult, setEvalCaptureResult] = useState<string | null>(
    null,
  );
  const [evalCaptureError, setEvalCaptureError] = useState<string | null>(null);

  // --- OCR Processing ---
  const [ocrFile, setOcrFile] = useState<File | null>(null);
  const [ocrLoading, setOcrLoading] = useState<boolean>(false);
  const [ocrResult, setOcrResult] = useState<OcrResponseEvent | null>(null);
  const [ocrError, setOcrError] = useState<string | null>(null);

  // --- Embedding Test ---
  const [embeddingInput, setEmbeddingInput] = useState<string>("");
  const [embeddingArray, setEmbeddingArray] = useState<number[] | null>(null);
  const [embeddingLoading, setEmbeddingLoading] = useState<boolean>(false);
  const [embeddingError, setEmbeddingError] = useState<string | null>(null);

  // --- Computer Use Test ---
  const [computerUsePrompt, setComputerUsePrompt] = useState<string>(
    "What is the capital of France?",
  );
  const [computerUseResult, setComputerUseResult] = useState<string | null>(
    null,
  );
  const [computerUseLoading, setComputerUseLoading] = useState<boolean>(false);
  const [computerUseError, setComputerUseError] = useState<string | null>(null);

  // --- Computer Use Action Testing ---
  const [selectedAction, setSelectedAction] =
    useState<string>("OpenWebBrowser");
  const [actionInputs, setActionInputs] = useState<{
    url: string;
    x: string | number;
    y: string | number;
    text: string;
    press_enter: boolean;
    clear_before_typing: boolean;
    keys: string;
    direction: string;
    magnitude: string | number;
    destination_x: string | number;
    destination_y: string | number;
  }>({
    url: "https://www.google.com",
    x: 500,
    y: 500,
    text: "Hello World",
    press_enter: true,
    clear_before_typing: true,
    keys: "control+a",
    direction: "down",
    magnitude: 800,
    destination_x: 600,
    destination_y: 600,
  });
  const [actionOutput, setActionOutput] = useState<unknown>(null);
  const [actionLoading, setActionLoading] = useState<boolean>(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const handleGenerateEmbedding = async () => {
    if (!embeddingInput.trim()) return;
    setEmbeddingLoading(true);
    setEmbeddingError(null);
    setEmbeddingArray(null);
    try {
      const result = await invoke<number[]>("generate_embedding", {
        input: embeddingInput,
      });
      setEmbeddingArray(result);
    } catch (err) {
      setEmbeddingError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setEmbeddingLoading(false);
    }
  };

  const handleTestComputerUse = async () => {
    if (!computerUsePrompt.trim()) return;
    setComputerUseLoading(true);
    setComputerUseError(null);
    setComputerUseResult(null);
    try {
      const result = await invoke<string>("start_computer_use", {
        prompt: computerUsePrompt,
      });
      setComputerUseResult(result);
      console.log("Computer Use Result:", result);
    } catch (err) {
      setComputerUseError(typeof err === "string" ? err : JSON.stringify(err));
      console.error("Error testing computer use:", err);
    } finally {
      setComputerUseLoading(false);
    }
  };

  const handleExecuteAction = async () => {
    setActionLoading(true);
    setActionOutput(null);
    setActionError(null);

    let data: Record<string, unknown> | null = null;
    switch (selectedAction) {
      case "Navigate":
        data = { url: actionInputs.url };
        break;
      case "ClickAt":
        data = {
          x: Number.parseInt(String(actionInputs.x)),
          y: Number.parseInt(String(actionInputs.y)),
        };
        break;
      case "HoverAt":
        data = {
          x: Number.parseInt(String(actionInputs.x)),
          y: Number.parseInt(String(actionInputs.y)),
        };
        break;
      case "TypeTextAt":
        data = {
          x: Number.parseInt(String(actionInputs.x)),
          y: Number.parseInt(String(actionInputs.y)),
          text: actionInputs.text,
          press_enter: actionInputs.press_enter,
          clear_before_typing: actionInputs.clear_before_typing,
        };
        break;
      case "KeyCombination":
        data = { keys: actionInputs.keys };
        break;
      case "ScrollDocument":
        data = { direction: actionInputs.direction };
        break;
      case "ScrollAt":
        data = {
          x: Number.parseInt(String(actionInputs.x)),
          y: Number.parseInt(String(actionInputs.y)),
          direction: actionInputs.direction,
          magnitude: Number.parseInt(String(actionInputs.magnitude)),
        };
        break;
      case "DragAndDrop":
        data = {
          x: Number.parseInt(String(actionInputs.x)),
          y: Number.parseInt(String(actionInputs.y)),
          destination_x: Number.parseInt(String(actionInputs.destination_x)),
          destination_y: Number.parseInt(String(actionInputs.destination_y)),
        };
        break;
    }

    try {
      console.log(
        `Executing direct action: ${selectedAction} with data:`,
        data,
      );
      const result = await invoke("execute_computer_action", {
        action: data
          ? { action: selectedAction, data }
          : { action: selectedAction },
      });
      setActionOutput(result);
    } catch (err) {
      setActionError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setActionLoading(false);
    }
  };

  // --- Supabase user object ---
  const [supabaseUser, setSupabaseUser] = useState<SupabaseUser | null>(null);
  const [supabaseToken, setSupabaseToken] = useState<string | null>(null);

  const fetchSupabaseUser = async () => {
    try {
      const accessToken = await invoke<string>("get_access_token_command");
      console.log("Access Token:", accessToken);
      setSupabaseToken(accessToken);
      if (accessToken) {
        const supabaseUser = await invoke<SupabaseUser>("get_user", {
          accessToken,
        });
        console.log("Supabase User:", supabaseUser);
        setSupabaseUser(supabaseUser);
      }
    } catch (error) {
      console.error("Error fetching Supabase user:", error);
    }
  };

  // --- Screen Selection ---
  const [screenSelectionResult, setScreenSelectionResult] =
    useState<ScreenSelectionResult | null>(null);
  const [screenSelectionLoading, setScreenSelectionLoading] =
    useState<boolean>(false);

  const fetchScreenText = async () => {
    setScreenTextLoading(true);
    setScreenTextError(null);
    try {
      const data = await invoke<string>("get_screen_text_formatted");
      setScreenTextData(data);
    } catch (err) {
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
    } catch (err) {
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
      await invoke("open_screen_selector");
    } catch (error) {
      console.error("Failed to open screen selector:", error);
      setScreenSelectionLoading(false);
    }
  };

  // Listen for ocr results
  useEffect(() => {
    let unlistenStream: (() => void) | undefined;

    async function listenForOcrResults() {
      unlistenStream = await listen<OcrResponseEvent>(
        "ocr_response",
        (event) => {
          const { text } = event.payload;
          const result = event.payload;
          console.log("OCR result received:", text);
          setOcrResult(result);
        },
      );
    }

    void listenForOcrResults();

    return () => {
      if (unlistenStream) unlistenStream();
    };
  }, []);

  const handleFileUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) {
      // Check if file is an image
      if (file.type.startsWith("image/")) {
        setOcrFile(file);
        setOcrError(null);
        setOcrResult(null);
      } else {
        setOcrError("Please select a valid image file (PNG, JPG, JPEG)");
        setOcrFile(null);
      }
    }
  };

  const processOcrImage = async () => {
    if (!ocrFile) {
      setOcrError("Please select an image file first");
      return;
    }

    setOcrLoading(true);
    setOcrError(null);
    setOcrResult(null);

    try {
      // Convert file to byte array
      const arrayBuffer = await ocrFile.arrayBuffer();
      const imageData = Array.from(new Uint8Array(arrayBuffer));

      console.log(
        "Processing OCR for file:",
        ocrFile.name,
        "Size:",
        imageData.length,
        "bytes",
      );

      // Call the Tauri OCR command
      const result = await invoke<OcrResponseEvent>("process_image", {
        imageData,
      });

      setOcrResult(result);
      console.log("OCR processing completed:", result);
    } catch (err) {
      setOcrError(typeof err === "string" ? err : JSON.stringify(err));
      console.error("Error processing OCR:", err);
    } finally {
      setOcrLoading(false);
    }
  };

  return (
    <div className="relative flex flex-col items-center justify-center p-4 space-y-6 max-w-[30rem]">
      {/* Capture Scheduler Controls */}
      <div className="flex gap-4 justify-center">
        <Button
          onClick={() => {
            void handleStartScheduler();
          }}
          disabled={schedulerLoading || isSchedulerRunning}
          variant={isSchedulerRunning ? "secondary" : "default"}
        >
          {schedulerLoading && !isSchedulerRunning
            ? "Starting..."
            : "Start Capture Scheduler"}
        </Button>
        <Button
          onClick={() => {
            void handleStopScheduler();
          }}
          disabled={schedulerLoading || !isSchedulerRunning}
          variant={isSchedulerRunning ? "destructive" : "secondary"}
        >
          {schedulerLoading && isSchedulerRunning
            ? "Stopping..."
            : "Stop Capture Scheduler"}
        </Button>
      </div>

      {/* Status indicator */}
      <div className="text-sm text-center">
        Status:{" "}
        <span
          className={
            isSchedulerRunning
              ? "text-green-600 font-semibold"
              : "text-red-600 font-semibold"
          }
        >
          {isSchedulerRunning ? "Running" : "Stopped"}
        </span>
      </div>

      {/* Screen Text Button */}
      <div className="flex justify-center">
        <Button
          onClick={() => {
            void fetchScreenText();
          }}
          disabled={screenTextLoading}
        >
          {screenTextLoading ? "Loading..." : "Get Screen Text (Formatted)"}
        </Button>
      </div>

      {/* Open and close computer use window */}
      <div className="flex gap-4 justify-center">
        <Button
          onClick={() => {
            void (async () => {
              await invoke("open_computer_use_window");
            })();
          }}
          variant="default"
        >
          Open Computer Use Window
        </Button>
        <Button
          onClick={() => {
            void (async () => {
              await invoke("close_computer_use_window");
            })();
          }}
          variant="destructive"
        >
          Close Computer Use Window
        </Button>
      </div>

      {/* Screen Selection Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-blue-50">
        <h2 className="text-lg font-semibold">Screen Selection Tool</h2>
        <p className="text-sm text-gray-600">
          Click to open a fullscreen overlay where you can select any area of
          your screen to extract text from that specific region.
        </p>
        <Button
          onClick={() => {
            void openScreenSelector();
          }}
          disabled={screenSelectionLoading}
          variant="default"
        >
          {screenSelectionLoading
            ? "Select an area..."
            : "📱 Select Screen Area"}
        </Button>

        {screenSelectionResult && (
          <div className="mt-4 space-y-2">
            <h3 className="text-md font-semibold">Selection Result:</h3>
            <div className="p-2 bg-gray-100 rounded text-sm">
              <strong>Bounds:</strong> {screenSelectionResult.bounds.width}x
              {screenSelectionResult.bounds.height} at (
              {screenSelectionResult.bounds.x}, {screenSelectionResult.bounds.y}
              )
            </div>
            <div className="p-2 bg-white border rounded text-sm max-h-64 overflow-y-auto">
              <strong>Extracted Text:</strong>
              <pre className="whitespace-pre-wrap mt-2">
                {screenSelectionResult.text_content ||
                  "No text found in selected area"}
              </pre>
            </div>
          </div>
        )}
      </div>

      {/* Evaluation Data Capture Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-orange-50">
        <h2 className="text-lg font-semibold">Evaluation Data Capture</h2>
        <p className="text-sm text-gray-600">
          Click this button when you see incorrect task detection to save the
          current state for evaluation. Requires at least 2 screen captures and
          active tasks.
        </p>
        <Button
          onClick={() => {
            void captureEvalData();
          }}
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

      {/* OCR Processing Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-blue-50">
        <h2 className="text-lg font-semibold">OCR Text Extraction</h2>

        <div className="space-y-2">
          <Label htmlFor="ocr-file">Select Image File</Label>
          <Input
            id="ocr-file"
            type="file"
            accept="image/*"
            onChange={handleFileUpload}
          />
        </div>

        {ocrFile && (
          <div className="text-sm text-gray-600">
            Selected: {ocrFile.name} ({(ocrFile.size / 1024).toFixed(1)} KB)
          </div>
        )}

        <Button
          onClick={() => {
            void processOcrImage();
          }}
          disabled={ocrLoading || !ocrFile}
          variant="default"
        >
          {ocrLoading ? "Processing..." : "Extract Text"}
        </Button>

        {ocrResult && (
          <div className="mt-4 space-y-2">
            <h3 className="text-md font-semibold">OCR Results:</h3>
            <div className="mt-2">
              <Label>Extracted Text:</Label>
              <Textarea
                value={ocrResult.text}
                readOnly
                rows={6}
                className="mt-1 font-mono text-sm"
                placeholder="Extracted text will appear here..."
              />
            </div>
          </div>
        )}

        {ocrError && (
          <div className="mt-2 p-2 bg-red-100 border border-red-300 rounded text-sm">
            <strong>Error:</strong> {ocrError}
          </div>
        )}
      </div>

      {/* Embedding Test Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-purple-50">
        <h2 className="text-lg font-semibold">Embedding Test</h2>
        <p className="text-sm text-gray-600">
          Enter text to generate an embedding using the local model.
        </p>
        <Textarea
          value={embeddingInput}
          onChange={(e) => {
            setEmbeddingInput(e.target.value);
          }}
          rows={3}
          placeholder="Type a sentence or short paragraph..."
        />
        <div className="flex items-center gap-3 flex-wrap">
          <Button
            onClick={() => {
              void handleGenerateEmbedding();
            }}
            disabled={embeddingLoading || !embeddingInput.trim()}
            variant="default"
          >
            {embeddingLoading ? "Generating..." : "Generate Embedding"}
          </Button>
          {embeddingArray && (
            <span className="text-xs text-gray-700">
              Dims: {embeddingArray.length}
            </span>
          )}
        </div>
        {embeddingError && (
          <div className="p-2 bg-red-100 border border-red-300 rounded text-xs font-mono overflow-x-auto">
            Error: {embeddingError}
          </div>
        )}
        {embeddingArray && !embeddingError && (
          <pre className="p-2 bg-white border rounded text-[10px] leading-tight max-h-40 overflow-y-auto whitespace-pre-wrap break-words">
            {embeddingArray
              .slice(0, 64)
              .map((n) => n.toFixed(4))
              .join(", ")}
            {embeddingArray.length > 64 ? " ..." : ""}
          </pre>
        )}
      </div>

      {/* Computer Use Test Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-green-50">
        <h2 className="text-lg font-semibold">Computer Use Engine Test</h2>
        <p className="text-sm text-gray-600">
          Test the Gemini Computer Use API with a custom prompt.
        </p>
        <Textarea
          value={computerUsePrompt}
          onChange={(e) => {
            setComputerUsePrompt(e.target.value);
          }}
          rows={3}
          placeholder="Enter a prompt for the computer use engine..."
        />
        <Button
          onClick={() => {
            void handleTestComputerUse();
          }}
          disabled={computerUseLoading || !computerUsePrompt.trim()}
          variant="default"
        >
          {computerUseLoading ? "Testing..." : "Test Computer Use"}
        </Button>
        {computerUseError && (
          <div className="p-2 bg-red-100 border border-red-300 rounded text-xs font-mono overflow-x-auto">
            Error: {computerUseError}
          </div>
        )}
        {computerUseResult && !computerUseError && (
          <div className="space-y-2">
            <Label>API Response:</Label>
            <pre className="p-3 bg-white border rounded text-xs leading-relaxed max-h-96 overflow-y-auto whitespace-pre-wrap break-words">
              {computerUseResult}
            </pre>
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
            onChange={(e) => {
              setSqlQuery(e.target.value);
            }}
            placeholder="Enter SQL query (e.g., SELECT * FROM documents)"
            rows={4}
          />
        </div>
        <div className="space-y-2">
          <Label htmlFor="sql-params">Parameters (JSON Array)</Label>
          <Textarea
            id="sql-params"
            value={sqlParams}
            onChange={(e) => {
              setSqlParams(e.target.value);
            }}
            placeholder='Enter parameters as JSON array (e.g., ["value1", 123]) or leave empty'
            rows={2}
          />
        </div>
        <Button
          onClick={() => {
            void handleExecuteSql();
          }}
        >
          Execute SQL
        </Button>
        {(sqlResult || sqlError) && (
          <div className="mt-4">
            <h3 className="text-md font-semibold">Result:</h3>
            <pre className="mt-2 p-2 border rounded bg-gray-50 text-sm overflow-x-auto">
              {sqlError ? `Error: ${sqlError}` : sqlResult}
            </pre>
          </div>
        )}
      </div>

      {/* Direct Computer Action Test Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-red-50">
        <h2 className="text-lg font-semibold">Direct Computer Action Test</h2>
        <p className="text-sm text-gray-600">
          Test individual computer use actions directly.
        </p>

        <div className="space-y-2">
          <Label htmlFor="action-select">Select Action</Label>
          <select
            id="action-select"
            className="w-full p-2 border rounded-md bg-white text-sm"
            value={selectedAction}
            onChange={(e) => {
              setSelectedAction(e.target.value);
            }}
          >
            <option value="OpenWebBrowser">Open Web Browser</option>
            <option value="Wait5Seconds">Wait 5 Seconds</option>
            <option value="GoBack">Go Back</option>
            <option value="GoForward">Go Forward</option>
            <option value="Search">Search</option>
            <option value="Navigate">Navigate</option>
            <option value="ClickAt">Click At</option>
            <option value="HoverAt">Hover At</option>
            <option value="TypeTextAt">Type Text At</option>
            <option value="KeyCombination">Key Combination</option>
            <option value="ScrollDocument">Scroll Document</option>
            <option value="ScrollAt">Scroll At</option>
            <option value="DragAndDrop">Drag and Drop</option>
          </select>
        </div>

        {/* Dynamic Inputs based on selected action */}
        <div className="grid grid-cols-2 gap-4">
          {selectedAction === "Navigate" && (
            <div className="col-span-2 space-y-2">
              <Label>URL</Label>
              <Input
                value={actionInputs.url}
                onChange={(e) => {
                  setActionInputs({ ...actionInputs, url: e.target.value });
                }}
              />
            </div>
          )}

          {(selectedAction === "ClickAt" ||
            selectedAction === "HoverAt" ||
            selectedAction === "TypeTextAt" ||
            selectedAction === "ScrollAt" ||
            selectedAction === "DragAndDrop") && (
            <>
              <div className="space-y-2">
                <Label>X Coordinate</Label>
                <Input
                  type="number"
                  value={actionInputs.x}
                  onChange={(e) => {
                    setActionInputs({ ...actionInputs, x: e.target.value });
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label>Y Coordinate</Label>
                <Input
                  type="number"
                  value={actionInputs.y}
                  onChange={(e) => {
                    setActionInputs({ ...actionInputs, y: e.target.value });
                  }}
                />
              </div>
            </>
          )}

          {selectedAction === "TypeTextAt" && (
            <>
              <div className="col-span-2 space-y-2">
                <Label>Text to Type</Label>
                <Input
                  value={actionInputs.text}
                  onChange={(e) => {
                    setActionInputs({ ...actionInputs, text: e.target.value });
                  }}
                />
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={actionInputs.press_enter}
                  onChange={(e) => {
                    setActionInputs({
                      ...actionInputs,
                      press_enter: e.target.checked,
                    });
                  }}
                />
                <Label>Press Enter</Label>
              </div>
              <div className="flex items-center gap-2">
                <input
                  type="checkbox"
                  checked={actionInputs.clear_before_typing}
                  onChange={(e) => {
                    setActionInputs({
                      ...actionInputs,
                      clear_before_typing: e.target.checked,
                    });
                  }}
                />
                <Label>Clear Before Typing</Label>
              </div>
            </>
          )}

          {selectedAction === "KeyCombination" && (
            <div className="col-span-2 space-y-2">
              <Label>Keys (e.g. control+c)</Label>
              <Input
                value={actionInputs.keys}
                onChange={(e) => {
                  setActionInputs({ ...actionInputs, keys: e.target.value });
                }}
              />
            </div>
          )}

          {(selectedAction === "ScrollDocument" ||
            selectedAction === "ScrollAt") && (
            <div className="space-y-2">
              <Label>Direction</Label>
              <select
                className="w-full p-2 border rounded-md text-sm"
                value={actionInputs.direction}
                onChange={(e) => {
                  setActionInputs({
                    ...actionInputs,
                    direction: e.target.value,
                  });
                }}
              >
                <option value="up">Up</option>
                <option value="down">Down</option>
                <option value="left">Left</option>
                <option value="right">Right</option>
              </select>
            </div>
          )}

          {selectedAction === "ScrollAt" && (
            <div className="space-y-2">
              <Label>Magnitude</Label>
              <Input
                type="number"
                value={actionInputs.magnitude}
                onChange={(e) => {
                  setActionInputs({
                    ...actionInputs,
                    magnitude: e.target.value,
                  });
                }}
              />
            </div>
          )}

          {selectedAction === "DragAndDrop" && (
            <>
              <div className="space-y-2">
                <Label>Dest X</Label>
                <Input
                  type="number"
                  value={actionInputs.destination_x}
                  onChange={(e) => {
                    setActionInputs({
                      ...actionInputs,
                      destination_x: e.target.value,
                    });
                  }}
                />
              </div>
              <div className="space-y-2">
                <Label>Dest Y</Label>
                <Input
                  type="number"
                  value={actionInputs.destination_y}
                  onChange={(e) => {
                    setActionInputs({
                      ...actionInputs,
                      destination_y: e.target.value,
                    });
                  }}
                />
              </div>
            </>
          )}
        </div>

        <Button
          onClick={() => {
            void handleExecuteAction();
          }}
          disabled={actionLoading}
          variant="destructive"
          className="w-full"
        >
          {actionLoading ? "Executing..." : `Execute ${selectedAction}`}
        </Button>

        {!!actionError && (
          <div className="mt-2 p-2 bg-red-100 border border-red-300 rounded text-sm whitespace-pre-wrap">
            Error: {String(actionError)}
          </div>
        )}

        {!!actionOutput && (
          <div className="mt-2 p-2 bg-green-100 border border-green-300 rounded text-sm overflow-auto max-h-60">
            <h3 className="font-semibold mb-1 text-xs">Result:</h3>
            <pre className="text-[10px] leading-tight">
              {JSON.stringify(actionOutput, null, 2)}
            </pre>
          </div>
        )}
      </div>

      {/* Supabase user fetching */}
      <div className="w-full max-w-4xl mt-4 p-4 border rounded-md bg-yellow-50 space-y-4">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">Supabase Auth</h2>
          <Button
            onClick={() => {
              void fetchSupabaseUser();
            }}
            variant="default"
          >
            Refresh Auth Info
          </Button>
        </div>

        <div className="space-y-2">
          <h3 className="text-sm font-medium">Access Token</h3>
          <div className="flex gap-2">
            <Input
              readOnly
              value={supabaseToken || "No token fetched"}
              className="bg-white font-mono text-xs"
            />
            <Button
              variant="outline"
              size="sm"
              onClick={() => {
                if (supabaseToken) {
                  void navigator.clipboard.writeText(supabaseToken);
                }
              }}
              disabled={!supabaseToken}
            >
              Copy
            </Button>
          </div>
        </div>

        <div className="space-y-2">
          <h3 className="text-sm font-medium">User Profile</h3>
          <pre className="whitespace-pre-wrap text-sm bg-white p-2 rounded border max-h-60 overflow-auto">
            {supabaseUser
              ? JSON.stringify(supabaseUser, null, 2)
              : "No user data fetched"}
          </pre>
        </div>
      </div>

      {/* --- Screen Text by Application Section --- */}
      <div className="w-full max-w-4xl mt-4 p-4 border rounded-md bg-yellow-50">
        <h2 className="text-lg font-semibold mb-2">
          Screen Text by Application (Formatted)
        </h2>
        {screenTextData && (
          <div className="mt-2 prose prose-sm max-w-none max-h-96 overflow-y-auto bg-white p-4 rounded border">
            <pre className="whitespace-pre-wrap text-sm">{screenTextData}</pre>
          </div>
        )}
        {screenTextError && (
          <div className="mt-2 text-red-700 font-mono">
            Error: {screenTextError}
          </div>
        )}
        {!screenTextData && !screenTextError && !screenTextLoading && (
          <div className="mt-2 text-gray-500">
            Click &quot;Get Screen Text (Formatted)&quot; button to fetch clean,
            organized application text data.
          </div>
        )}
        {screenTextLoading && (
          <div className="mt-2 text-blue-600">
            Loading screen text data (this may take a moment)...
          </div>
        )}
      </div>
    </div>
  );
}
