"use client";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { useRoleAccessContext } from "@/lib/role-access/RoleAccessProvider";
import type { SupabaseUser } from "@/types/auth";
import type { OcrResponseEvent } from "@/types/events";
import type { SkillSummary, ToolDefinition, ToolResult } from "@/types/skills";
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
  const { state: authState } = useRoleAccessContext();
  // State for SQL execution
  const [sqlQuery, setSqlQuery] = useState<string>(
    "SELECT * FROM documents LIMIT 5;",
  );
  const [sqlParams, setSqlParams] = useState<string>("[]"); // Store params as JSON string
  const [sqlResult, setSqlResult] = useState<string | null>(null);
  const [sqlError, setSqlError] = useState<string | null>(null);

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

  // --- Skill Tool Tester ---
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [selectedSkill, setSelectedSkill] = useState<string>("");
  const [availableTools, setAvailableTools] = useState<ToolDefinition[]>([]);
  const [selectedTool, setSelectedTool] = useState<string>("");
  const [toolArguments, setToolArguments] = useState<string>("{}");
  const [toolResult, setToolResult] = useState<ToolResult | null>(null);
  const [toolLoading, setToolLoading] = useState<boolean>(false);
  const [toolError, setToolError] = useState<string | null>(null);

  useEffect(() => {
    const fetchSkills = async () => {
      try {
        const result = await invoke<SkillSummary[]>("get_available_skills");
        setSkills(result);
      } catch (err) {
        console.error("Failed to fetch skills:", err);
      }
    };
    void fetchSkills();
  }, []);

  useEffect(() => {
    const fetchTools = async () => {
      if (!selectedSkill) {
        setAvailableTools([]);
        return;
      }
      try {
        const result = await invoke<ToolDefinition[]>(
          "get_skill_tools_command",
          {
            name: selectedSkill,
          },
        );
        setAvailableTools(result);
        if (result.length > 0) {
          setSelectedTool(result[0].name);
        } else {
          setSelectedTool("");
        }
      } catch (err) {
        console.error("Failed to fetch tools:", err);
      }
    };
    void fetchTools();
  }, [selectedSkill]);

  useEffect(() => {
    if (selectedTool) {
      const tool = availableTools.find((t) => t.name === selectedTool);
      if (tool) {
        const template: Record<string, unknown> = {};
        for (const param of tool.parameters) {
          template[param.name] =
            param.default ??
            (param.type === "string"
              ? ""
              : param.type === "number" || param.type === "integer"
                ? 0
                : param.type === "boolean"
                  ? false
                  : null);
        }
        setToolArguments(JSON.stringify(template, null, 2));
      }
    }
  }, [selectedTool, availableTools]);

  const handleExecuteTool = async () => {
    if (!selectedSkill || !selectedTool) return;
    setToolLoading(true);
    setToolError(null);
    setToolResult(null);

    let parsedArgs: Record<string, unknown> = {};
    try {
      parsedArgs = JSON.parse(toolArguments) as Record<string, unknown>;
    } catch (e) {
      setToolError(
        `Invalid JSON in arguments: ${e instanceof Error ? e.message : String(e)}`,
      );
      setToolLoading(false);
      return;
    }

    try {
      const result = await invoke<ToolResult>("execute_skill_tool", {
        skillName: selectedSkill,
        toolName: selectedTool,
        arguments: parsedArgs,
      });
      setToolResult(result);
    } catch (err) {
      setToolError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setToolLoading(false);
    }
  };

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

  // --- Browser Use Testing ---
  const [browserUrl, setBrowserUrl] = useState<string>(
    "https://www.google.com",
  );
  const [browserCreated, setBrowserCreated] = useState<boolean>(false);
  const [browserSnapshot, setBrowserSnapshot] = useState<string | null>(null);
  const [browserActionName, setBrowserActionName] =
    useState<string>("navigate");
  const [browserActionArgs, setBrowserActionArgs] = useState<string>(
    '{"url": "https://www.google.com"}',
  );
  const [browserActionResult, setBrowserActionResult] = useState<string | null>(
    null,
  );
  const [browserLoading, setBrowserLoading] = useState<boolean>(false);
  const [browserError, setBrowserError] = useState<string | null>(null);

  const browserActions = [
    "navigate",
    "click",
    "type_text",
    "select_option",
    "scroll",
    "go_back",
    "wait",
  ];

  const browserActionTemplates: Record<string, string> = {
    navigate: '{"url": "https://www.google.com"}',
    click: '{"element_id": 1}',
    type_text: '{"element_id": 1, "text": "hello", "press_enter": true}',
    select_option: '{"element_id": 1, "value": "option1"}',
    scroll: '{"direction": "down"}',
    go_back: "{}",
    wait: '{"seconds": 2}',
  };

  const handleBrowserCreate = async () => {
    setBrowserLoading(true);
    setBrowserError(null);
    try {
      await invoke<string>("browser_test_create", { url: browserUrl });
      setBrowserCreated(true);
    } catch (err) {
      setBrowserError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setBrowserLoading(false);
    }
  };

  const handleBrowserSnapshot = async () => {
    setBrowserLoading(true);
    setBrowserError(null);
    setBrowserSnapshot(null);
    try {
      const result = await invoke<string>("browser_test_snapshot");
      setBrowserSnapshot(result);
    } catch (err) {
      setBrowserError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setBrowserLoading(false);
    }
  };

  const handleBrowserAction = async () => {
    setBrowserLoading(true);
    setBrowserError(null);
    setBrowserActionResult(null);
    try {
      const args = JSON.parse(browserActionArgs) as Record<string, unknown>;
      const result = await invoke<string>("browser_test_action", {
        action: browserActionName,
        arguments: args,
      });
      setBrowserActionResult(result);
    } catch (err) {
      setBrowserError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setBrowserLoading(false);
    }
  };

  const handleBrowserActionThenSnapshot = async () => {
    setBrowserLoading(true);
    setBrowserError(null);
    setBrowserActionResult(null);
    setBrowserSnapshot(null);
    try {
      const args = JSON.parse(browserActionArgs) as Record<string, unknown>;
      const actionResult = await invoke<string>("browser_test_action", {
        action: browserActionName,
        arguments: args,
      });
      setBrowserActionResult(actionResult);

      // Wait for page to settle
      const delay = browserActionName === "navigate" ? 2500 : 1500;
      await new Promise((resolve) => setTimeout(resolve, delay));

      // Take snapshot
      const snapshot = await invoke<string>("browser_test_snapshot");
      setBrowserSnapshot(snapshot);
    } catch (err) {
      setBrowserError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setBrowserLoading(false);
    }
  };

  const handleBrowserDestroy = async () => {
    setBrowserLoading(true);
    setBrowserError(null);
    try {
      await invoke("browser_test_destroy");
      setBrowserCreated(false);
      setBrowserSnapshot(null);
      setBrowserActionResult(null);
    } catch (err) {
      setBrowserError(typeof err === "string" ? err : JSON.stringify(err));
    } finally {
      setBrowserLoading(false);
    }
  };

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
      {/* Google Auth Debug Info */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-yellow-50">
        <h2 className="text-lg font-semibold">Google Auth Status</h2>
        <div className="space-y-2 text-sm">
          <div>
            <strong>Is Logged In (Supabase):</strong>{" "}
            {authState.isLoggedIn ? "✅ Yes" : "❌ No"}
          </div>
          <div>
            <strong>Is Google Authenticated:</strong>{" "}
            {authState.isGoogleAuthenticated ? "✅ Yes" : "❌ No"}
          </div>
        </div>
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

      {/* Skill Tool Tester Section */}
      <div className="w-full max-w-2xl p-4 border rounded-md space-y-4 bg-orange-50">
        <h2 className="text-lg font-semibold">Skill Tool Tester</h2>
        <p className="text-sm text-gray-600">
          Test any tool from the skill registry with custom arguments.
        </p>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="skill-select">Skill</Label>
            <select
              id="skill-select"
              className="w-full p-2 border rounded bg-white text-sm"
              value={selectedSkill}
              onChange={(e) => {
                setSelectedSkill(e.target.value);
              }}
            >
              <option value="">Select a skill...</option>
              {skills.map((skill) => (
                <option key={skill.name} value={skill.name}>
                  {skill.name}
                </option>
              ))}
            </select>
          </div>

          <div className="space-y-2">
            <Label htmlFor="tool-select">Tool</Label>
            <select
              id="tool-select"
              className="w-full p-2 border rounded bg-white text-sm"
              value={selectedTool}
              disabled={!selectedSkill}
              onChange={(e) => {
                setSelectedTool(e.target.value);
              }}
            >
              <option value="">Select a tool...</option>
              {availableTools.map((tool) => (
                <option key={tool.name} value={tool.name}>
                  {tool.name}
                </option>
              ))}
            </select>
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor="tool-args">Arguments (JSON)</Label>
          <Textarea
            id="tool-args"
            value={toolArguments}
            onChange={(e) => {
              setToolArguments(e.target.value);
            }}
            rows={8}
            placeholder="Enter tool arguments as JSON..."
            className="font-mono text-xs"
          />
        </div>

        <Button
          onClick={() => {
            void handleExecuteTool();
          }}
          disabled={toolLoading || !selectedSkill || !selectedTool}
          variant="default"
        >
          {toolLoading ? "Executing..." : "Execute Tool"}
        </Button>

        {toolError && (
          <div className="p-2 bg-red-100 border border-red-300 rounded text-xs font-mono overflow-x-auto">
            Error: {toolError}
          </div>
        )}

        {toolResult && (
          <div className="space-y-2">
            <Label>Result:</Label>
            <div
              className={`p-3 border rounded text-xs leading-relaxed max-h-96 overflow-y-auto whitespace-pre-wrap break-words font-mono ${toolResult.success ? "bg-white" : "bg-red-50"}`}
            >
              {toolResult.success ? (
                <pre>{JSON.stringify(toolResult.result, null, 2)}</pre>
              ) : (
                <p className="text-red-600">
                  Execution Failed: {toolResult.error}
                </p>
              )}
            </div>
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

      {/* Browser Use Testing */}
      <div className="w-full max-w-4xl mt-4 p-4 border rounded-md bg-blue-50 space-y-4">
        <h2 className="text-lg font-semibold">Browser Use Testing</h2>

        {/* Create/Destroy WebView */}
        <div className="space-y-2">
          <Label>Start URL</Label>
          <div className="flex gap-2">
            <Input
              value={browserUrl}
              onChange={(e) => {
                setBrowserUrl(e.target.value);
              }}
              placeholder="https://www.google.com"
              className="bg-white"
            />
            {!browserCreated ? (
              <Button
                onClick={() => {
                  void handleBrowserCreate();
                }}
                disabled={browserLoading}
                className="shrink-0"
              >
                {browserLoading ? "Creating..." : "Create WebView"}
              </Button>
            ) : (
              <Button
                onClick={() => {
                  void handleBrowserDestroy();
                }}
                disabled={browserLoading}
                variant="destructive"
                className="shrink-0"
              >
                Destroy
              </Button>
            )}
          </div>
          {browserCreated && (
            <p className="text-xs text-green-700">WebView active</p>
          )}
        </div>

        {/* Snapshot */}
        {browserCreated && (
          <div className="space-y-2">
            <div className="flex gap-2">
              <Button
                onClick={() => {
                  void handleBrowserSnapshot();
                }}
                disabled={browserLoading}
                variant="outline"
                className="w-full"
              >
                {browserLoading ? "Extracting..." : "Get Snapshot"}
              </Button>
            </div>
          </div>
        )}

        {/* Actions */}
        {browserCreated && (
          <div className="space-y-2">
            <Label>Action</Label>
            <select
              className="w-full border rounded px-2 py-1.5 text-sm bg-white"
              value={browserActionName}
              onChange={(e) => {
                setBrowserActionName(e.target.value);
                setBrowserActionArgs(
                  browserActionTemplates[e.target.value] || "{}",
                );
              }}
            >
              {browserActions.map((a) => (
                <option key={a} value={a}>
                  {a}
                </option>
              ))}
            </select>

            <Label>Arguments (JSON)</Label>
            <Textarea
              value={browserActionArgs}
              onChange={(e) => {
                setBrowserActionArgs(e.target.value);
              }}
              className="bg-white font-mono text-xs"
              rows={3}
            />

            <div className="flex gap-2">
              <Button
                onClick={() => {
                  void handleBrowserAction();
                }}
                disabled={browserLoading}
                variant="outline"
                className="flex-1"
              >
                Execute Action
              </Button>
              <Button
                onClick={() => {
                  void handleBrowserActionThenSnapshot();
                }}
                disabled={browserLoading}
                className="flex-1"
              >
                Action + Snapshot
              </Button>
            </div>
          </div>
        )}

        {/* Action Result */}
        {browserActionResult && (
          <div className="p-2 bg-green-100 border border-green-300 rounded text-sm">
            <h3 className="font-semibold mb-1 text-xs">Action Result:</h3>
            <pre className="text-xs whitespace-pre-wrap">
              {browserActionResult}
            </pre>
          </div>
        )}

        {/* Snapshot Result */}
        {browserSnapshot && (
          <div className="p-2 bg-white border rounded text-sm overflow-auto max-h-96">
            <div className="flex items-center justify-between mb-1">
              <h3 className="font-semibold text-xs">
                Snapshot ({browserSnapshot.length} chars, ~
                {Math.ceil(browserSnapshot.length / 4)} tokens)
              </h3>
              <Button
                variant="ghost"
                size="sm"
                className="h-6 text-xs"
                onClick={() => {
                  void navigator.clipboard.writeText(browserSnapshot);
                }}
              >
                Copy
              </Button>
            </div>
            <pre className="text-[10px] leading-tight whitespace-pre-wrap font-mono">
              {browserSnapshot}
            </pre>
          </div>
        )}

        {/* Error */}
        {browserError && (
          <div className="p-2 bg-red-100 border border-red-300 rounded text-sm whitespace-pre-wrap">
            Error: {browserError}
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
    </div>
  );
}
