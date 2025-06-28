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

interface ConversationMessage {
  role: string;
  content: string;
}

interface StreamResponsePayload {
  content: string;
  is_finished: boolean;
  conversation_id: string;
}

export default function Home() {
  // Chat state
  const [chatInput, setChatInput] = useState("");  const [chatHistory, setChatHistory] = useState<{ sender: "user" | "bot"; text: string }[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [thinkingEnabled, setThinkingEnabled] = useState(false);
  const [streamingEnabled, setStreamingEnabled] = useState(false);
  const [isStreaming, setIsStreaming] = useState(false);
  const [currentStreamContent, setCurrentStreamContent] = useState("");
  const [modelStatus, setModelStatus] = useState<{initialized: boolean, loading: boolean}>({ initialized: false, loading: true });
  const chatEndRef = useRef<HTMLDivElement>(null);
  // Load conversation history on component mount
  useEffect(() => {
    loadConversationHistory();
    checkModelStatus();
    
    // Set up streaming listener
    let unlistenStream: (() => void) | undefined;
    
    async function setupStreamListener() {
      try {
        unlistenStream = await listen<StreamResponsePayload>('qwen3-stream', (event) => {
          const { content, is_finished, conversation_id } = event.payload;
          
          if (is_finished) {
            setIsStreaming(false);
            setCurrentStreamContent("");
            console.log('Stream finished for conversation:', conversation_id);
          } else {
            setCurrentStreamContent(prev => prev + content);
          }
        });
        console.log("Stream event listener set up.");
      } catch (error) {
        console.error("Failed to set up stream listener:", error);
      }
    }
    
    setupStreamListener();
    
    // Check model status periodically until initialized
    const statusInterval = setInterval(async () => {
      const isInitialized = await checkModelStatus();
      if (isInitialized) {
        clearInterval(statusInterval);
      }
    }, 2000);
    
    return () => {
      clearInterval(statusInterval);
      if (unlistenStream) {
        unlistenStream();
      }
    };
  }, []);

  // Check model initialization status
  async function checkModelStatus(): Promise<boolean> {
    try {
      const status = await invoke<{model_initialized: boolean, conversation_count: number}>("get_qwen3_status");
      setModelStatus({ 
        initialized: status.model_initialized, 
        loading: !status.model_initialized 
      });
      return status.model_initialized;
    } catch (err) {
      console.error("Error checking model status:", err);
      setModelStatus({ initialized: false, loading: true });
      return false;
    }
  }

  // Load conversation history from backend
  async function loadConversationHistory() {
    try {
      const history = await invoke<ConversationMessage[]>("get_conversation_history");
      const formattedHistory = history.map(msg => ({
        sender: msg.role === "User" ? "user" as const : "bot" as const,
        text: msg.content
      }));
      setChatHistory(formattedHistory);
      
      // Get current conversation ID
      const convId = await invoke<string | null>("get_current_conversation_id");
      setCurrentConversationId(convId);
    } catch (err) {
      console.log("No existing conversation history or error loading:", err);
    }
  }
  // Handle chat send
  async function handleSendChat(e?: React.FormEvent) {
    if (e) e.preventDefault();
    const prompt = chatInput.trim();
    if (!prompt || !modelStatus.initialized || isStreaming) return;
    
    setChatHistory((h) => [...h, { sender: "user", text: prompt }]);
    setChatInput("");
    
    if (streamingEnabled) {
      // Handle streaming
      setIsStreaming(true);
      setCurrentStreamContent("");
      
      // Add a placeholder for the bot response that will be updated in real-time
      setChatHistory((h) => [...h, { sender: "bot", text: "" }]);
      
      try {
        await invoke<string>("stream_qwen3", { 
          prompt,
          thinking: thinkingEnabled,
          resetConversation: false,
          conversationId: currentConversationId,
          systemPrompt: "You are a helpful assistant."
        });
        
        // Update conversation ID if this was the first message
        if (!currentConversationId) {
          const convId = await invoke<string | null>("get_current_conversation_id");
          setCurrentConversationId(convId);
        }
      } catch (err) {
        console.error("Error generating streaming response:", err);
        setChatHistory((h) => h.slice(0, -1).concat([{ sender: "bot", text: "[Error generating response]" }]));
        setIsStreaming(false);
        setCurrentStreamContent("");
      }
    } else {
      // Handle regular non-streaming
      try {
        const response = await invoke<string>("generate_qwen3", { 
          prompt,
          thinking: thinkingEnabled,
          resetConversation: false,
          conversationId: currentConversationId,
          systemPrompt: "You are a helpful assistant."
        });
        setChatHistory((h) => [...h, { sender: "bot", text: response }]);
        
        // Update conversation ID if this was the first message
        if (!currentConversationId) {
          const convId = await invoke<string | null>("get_current_conversation_id");
          setCurrentConversationId(convId);
        }
      } catch (err) {
        console.error("Error generating response:", err);
        setChatHistory((h) => [...h, { sender: "bot", text: "[Error generating response]" }]);
      }
    }
  }

  // Reset conversation
  async function resetConversation() {
    try {
      if (currentConversationId) {
        await invoke("reset_conversation", { conversationId: currentConversationId });
      }
      setChatHistory([]);
      setCurrentConversationId(null);
      setIsStreaming(false);
      setCurrentStreamContent("");
      console.log("Conversation reset successfully.");
    } catch (err) {
      console.error("Error resetting conversation:", err);
    }
  }

  // Scroll chat to bottom
  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [chatHistory]);

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
    <div className="relative flex flex-col items-center justify-center p-4">      {/* Chat Window */}
      <div className="w-full max-w-md mb-6 bg-white border rounded shadow p-4">        <div className="flex justify-between items-center mb-2">
          <h2 className="text-lg font-semibold">Chat with Qwen3</h2>
          <div className="flex items-center gap-2">
            {/* Model status indicator */}
            <div className="flex items-center text-xs">
              <div className={`w-2 h-2 rounded-full mr-1 ${
                modelStatus.initialized ? 'bg-green-500' : 
                modelStatus.loading ? 'bg-yellow-500 animate-pulse' : 'bg-red-500'
              }`}></div>
              <span className="text-gray-600">
                {modelStatus.initialized ? 'Ready' : 
                 modelStatus.loading ? 'Loading...' : 'Error'}
              </span>
            </div>
            <button
              onClick={resetConversation}
              className="text-sm bg-red-500 text-white px-2 py-1 rounded hover:bg-red-600"
            >
              Reset
            </button>
          </div>
        </div>
        
        {/* Thinking and Streaming toggles */}
        <div className="mb-2 space-y-1">
          <label className="flex items-center text-sm">
            <input
              type="checkbox"
              checked={thinkingEnabled}
              onChange={(e) => setThinkingEnabled(e.target.checked)}
              className="mr-2"
            />
            Enable thinking mode
          </label>
          <label className="flex items-center text-sm">
            <input
              type="checkbox"
              checked={streamingEnabled}
              onChange={(e) => setStreamingEnabled(e.target.checked)}
              className="mr-2"
            />
            Enable streaming responses
          </label>
        </div>
        
        <div className="h-48 overflow-y-auto mb-2 bg-gray-50 p-2 rounded">
          {chatHistory.length === 0 ? (
            <p className="text-gray-400">Start the conversation...</p>
          ) : (
            chatHistory.map((msg, idx) => {
              // For the last bot message during streaming, show the streaming content
              const isLastBotMessage = msg.sender === "bot" && idx === chatHistory.length - 1;
              const displayText = isLastBotMessage && isStreaming && currentStreamContent 
                ? currentStreamContent 
                : msg.text;
              
              return (
                <div key={idx} className={msg.sender === "user" ? "text-right mb-2" : "text-left mb-2"}>
                  <span className={msg.sender === "user" ? "text-blue-700" : "text-green-700"}>
                    <b>{msg.sender === "user" ? "You" : "Qwen3"}:</b> 
                    {isLastBotMessage && isStreaming && (
                      <span className="ml-1 text-xs text-gray-500">(streaming...)</span>
                    )}
                  </span>
                  <div className="mt-1 text-gray-800 whitespace-pre-wrap">
                    {displayText}
                    {isLastBotMessage && isStreaming && (
                      <span className="animate-pulse">▊</span>
                    )}
                  </div>
                </div>
              );
            })
          )}
          <div ref={chatEndRef} />
        </div>
        
        {/* Conversation info */}
        {currentConversationId && (
          <div className="text-xs text-gray-500 mb-2">
            Conversation ID: {currentConversationId.substring(0, 8)}...
          </div>
        )}
        
        <form className="flex gap-2" onSubmit={handleSendChat}>
          <input
            className="flex-1 border rounded px-2 py-1"
            type="text"
            value={chatInput}
            onChange={e => setChatInput(e.target.value)}
            placeholder="Type your message..."
            autoFocus
          />          <button
            className="bg-blue-600 text-white px-4 py-1 rounded hover:bg-blue-700 disabled:bg-gray-400"
            type="submit"
            disabled={!chatInput.trim() || !modelStatus.initialized || isStreaming}
          >
            {isStreaming ? 'Streaming...' : modelStatus.initialized ? 'Send' : 'Loading...'}
          </button>
        </form>
      </div>
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
