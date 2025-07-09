"use client";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event';
import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from "react";
import { Button } from "@/components/ui/button"
import Link from 'next/link'

// Define the expected payload structure
interface Message {
  id: string;
  conversation_id: string;
  role: string; // "user" or "assistant"
  content: string;
  timestamp: string;
}

interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

interface StreamResponsePayload {
  delta: string;
  is_finished: boolean;
  full_response: string;
}

export default function Home() {
  // Helper function to extract thinking content from text
  function extractThinkingContent(text: string) {
    const thinkStartIndex = text.indexOf('<think>');
    const thinkEndIndex = text.indexOf('</think>');
    
    let cleanText = text;
    let thinkingText = "";
    let isCurrentlyThinking = false;
    
    if (thinkStartIndex !== -1) {
      if (thinkEndIndex !== -1) {
        // Complete thinking block found
        thinkingText = text.substring(thinkStartIndex + 7, thinkEndIndex);
        cleanText = text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
      } else {
        // Thinking started but not finished yet
        thinkingText = text.substring(thinkStartIndex + 7);
        cleanText = text.substring(0, thinkStartIndex);
        isCurrentlyThinking = true;
      }
    }
    
    return { cleanText, thinkingText, isCurrentlyThinking };
  }

  // Chat state
  const [chatInput, setChatInput] = useState("");
  const [chatHistory, setChatHistory] = useState<{ sender: "user" | "bot"; text: string; thinking?: string }[]>([]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [thinkingEnabled, setThinkingEnabled] = useState(true);
  const [streamingEnabled, setStreamingEnabled] = useState(true);
  const [isStreaming, setIsStreaming] = useState(false);
  const [currentStreamContent, setCurrentStreamContent] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const chatEndRef = useRef<HTMLDivElement>(null);
  const streamContentRef = useRef<string>("");
  
  // Thinking state
  const [isThinking, setIsThinking] = useState(false);
  const [thinkingContent, setThinkingContent] = useState("");
  const [expandedThinking, setExpandedThinking] = useState<{[key: number]: boolean}>({});
  
  // Conversation management state
  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [sidebarOpen, setSidebarOpen] = useState(true);  // Load conversation history on component mount
  useEffect(() => {
    loadConversations();
    
    // Set up streaming listener
    let unlistenStream: (() => void) | undefined;
    
    async function setupStreamListener() {
      try {
        unlistenStream = await listen<StreamResponsePayload>('chat-stream', (event) => {
          const { delta, full_response, is_finished } = event.payload;
          
          console.log('Stream event:', { delta, is_finished, deltaLength: delta?.length });
          
          if (is_finished) {
            // When stream finishes, update the last bot message with the complete content
            setChatHistory((h) => {
              const newHistory = [...h];
              const lastIndex = newHistory.length - 1;
              if (lastIndex >= 0 && newHistory[lastIndex].sender === "bot") {
                // Extract thinking content if present
                const { cleanText, thinkingText } = extractThinkingContent(full_response);
                newHistory[lastIndex] = {
                  ...newHistory[lastIndex],
                  text: cleanText,
                  thinking: thinkingEnabled ? thinkingText : undefined
                };
              }
              return newHistory;
            });
            setIsStreaming(false);
            setIsThinking(false);
            setCurrentStreamContent("");
            setThinkingContent("");
            streamContentRef.current = "";
            
            // Reload conversations to update the sidebar
            loadConversations();
          } else {
            // Check if delta contains the full response or just incremental text
            // If it's full response, use it directly; if incremental, accumulate
            let contentToUse = "";
            if (full_response && full_response.length > 0) {
              // Backend is sending full response, use it directly
              streamContentRef.current = full_response;
              setCurrentStreamContent(full_response);
              contentToUse = full_response;
            } else {
              // Backend is sending incremental delta, accumulate it
              streamContentRef.current += delta;
              setCurrentStreamContent(streamContentRef.current);
              contentToUse = streamContentRef.current;
            }
            
            // Extract thinking content and clean text
            const { cleanText, thinkingText, isCurrentlyThinking } = extractThinkingContent(contentToUse);
            
            // Update thinking state
            if (thinkingEnabled) {
              setIsThinking(isCurrentlyThinking);
              setThinkingContent(thinkingText);
            }
            
            setChatHistory((h) => {
              const newHistory = [...h];
              const lastIndex = newHistory.length - 1;
              if (lastIndex >= 0 && newHistory[lastIndex].sender === "bot") {
                newHistory[lastIndex] = {
                  ...newHistory[lastIndex],
                  text: cleanText,
                  thinking: thinkingEnabled ? thinkingText : undefined
                };
              }
              return newHistory;
            });
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
      // Check if llama server is running
      try {
        const status = await invoke<boolean>("get_server_status");
        if (status) {
          clearInterval(statusInterval);
        }
      } catch (e) {
        console.log("Llama server not yet ready:", e);
      }
    }, 2000);
    
    return () => {
      clearInterval(statusInterval);
      if (unlistenStream) {
        unlistenStream();
      }
    };
  }, []);

  // Load all conversations for the sidebar
  async function loadConversations() {
    try {
      const convs = await invoke<Conversation[]>("list_conversations");
      setConversations(convs);
    } catch (err) {
      console.error("Error loading conversations:", err);
    }
  }

  // Load a specific conversation
  async function loadConversation(conversationId: string) {
    try {
      const messages = await invoke<Message[]>("get_messages", { conversationId });
      const formattedHistory = messages.map(msg => {
        const { cleanText, thinkingText } = extractThinkingContent(msg.content);
        return {
          sender: msg.role === "user" ? "user" as const : "bot" as const,
          text: cleanText,
          thinking: msg.role === "assistant" && thinkingText ? thinkingText : undefined
        };
      });
      setChatHistory(formattedHistory);
      setCurrentConversationId(conversationId);
    } catch (err) {
      console.error("Error loading conversation:", err);
    }
  }

  // Create a new conversation
  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      setChatHistory([]);
      setConversations(prev => [newConv, ...prev]);
    } catch (err) {
      console.error("Error creating conversation:", err);
    }
  }

  // Delete a conversation
  async function deleteConversation(conversationId: string) {
    try {
      await invoke("delete_conversation", { conversationId });
      setConversations(prev => prev.filter(c => c.id !== conversationId));
      
      // If deleting current conversation, clear it
      if (conversationId === currentConversationId) {
        setCurrentConversationId(null);
        setChatHistory([]);
      }
    } catch (err) {
      console.error("Error deleting conversation:", err);
    }
  }
  // Handle chat send with new generate function
  async function handleSendChat(e?: React.FormEvent) {
    if (e) e.preventDefault();
    const prompt = chatInput.trim();
    if (!prompt || isStreaming || isLoading) return;
    
    setChatHistory((h) => [...h, { sender: "user", text: prompt }]);
    setChatInput("");
    setIsLoading(true);
    
    // Create conversation if none exists
    let convId = currentConversationId;
    if (!convId) {
      try {
        const newConv = await invoke<Conversation>("create_conversation", { name: null });
        convId = newConv.id;
        setCurrentConversationId(convId);
        setConversations(prev => [newConv, ...prev]);
      } catch (err) {
        console.error("Error creating conversation:", err);
        setIsLoading(false);
        return;
      }
    }
    
    if (streamingEnabled) {
      // Handle streaming
      setIsStreaming(true);
      setCurrentStreamContent("");
      streamContentRef.current = "";
      
      // Add a placeholder for the bot response that will be updated in real-time
      setChatHistory((h) => [...h, { sender: "bot", text: "" }]);
      
      try {
        await invoke<string>("generate", { 
          prompt,
          jsonSchema: null,
          convId,
          useThinking: thinkingEnabled,
          stream: true
        });
      } catch (err) {
        console.error("Error generating streaming response:", err);
        setChatHistory((h) => h.slice(0, -1).concat([{ sender: "bot", text: "[Error generating response]" }]));
        setIsStreaming(false);
        setIsThinking(false);
        setCurrentStreamContent("");
        setThinkingContent("");
        streamContentRef.current = "";
      }
    } else {
      // Handle regular non-streaming
      try {
        const response = await invoke<string>("generate", { 
          prompt,
          jsonSchema: null,
          convId,
          useThinking: thinkingEnabled,
          stream: false
        });
        setChatHistory((h) => [...h, { sender: "bot", text: response }]);
        
        // Reload conversations to update the sidebar
        loadConversations();
      } catch (err) {
        console.error("Error generating response:", err);
        setChatHistory((h) => [...h, { sender: "bot", text: "[Error generating response]" }]);
      }
    }
    
    setIsLoading(false);
  }

  // Reset conversation
  async function resetConversation() {
    try {
      if (currentConversationId) {
        await invoke("reset_conversation", { conversationId: currentConversationId });
        loadConversations(); // Refresh sidebar
      }
      setChatHistory([]);
      setIsStreaming(false);
      setIsThinking(false);
      setCurrentStreamContent("");
      setThinkingContent("");
      streamContentRef.current = "";
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
        unlisten = await listen<any>('task-completed', (event) => {
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

  return (
    <div className="flex h-screen bg-gray-100">
      {/* Conversations Sidebar */}
      <div className={`${sidebarOpen ? 'w-64' : 'w-0'} transition-all duration-300 bg-white border-r overflow-hidden`}>
        <div className="p-4 border-b">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-semibold">Conversations</h2>
            <button
              onClick={() => setSidebarOpen(!sidebarOpen)}
              className="text-gray-500 hover:text-gray-700"
            >
              {sidebarOpen ? '←' : '→'}
            </button>
          </div>
          <button
            onClick={createNewConversation}
            className="w-full bg-blue-600 text-white px-3 py-2 rounded hover:bg-blue-700 flex items-center justify-center gap-2"
          >
            <span>+</span> New Chat
          </button>
        </div>
        
        <div className="flex-1 overflow-y-auto p-2">
          {conversations.map((conv) => (
            <div
              key={conv.id}
              className={`group flex items-center justify-between p-3 rounded cursor-pointer hover:bg-gray-100 ${
                currentConversationId === conv.id ? 'bg-blue-50 border-l-4 border-blue-600' : ''
              }`}
              onClick={() => loadConversation(conv.id)}
            >
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-gray-900 truncate">
                  {conv.name}
                </div>
                <div className="text-xs text-gray-500">
                  {conv.message_count} messages
                </div>
              </div>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  deleteConversation(conv.id);
                }}
                className="opacity-0 group-hover:opacity-100 text-red-500 hover:text-red-700 p-1"
              >
                ×
              </button>
            </div>
          ))}
        </div>
      </div>

      {/* Main Chat Area */}
      <div className="flex-1 flex flex-col">
        {/* Toggle sidebar button for mobile */}
        {!sidebarOpen && (
          <button
            onClick={() => setSidebarOpen(true)}
            className="absolute top-4 left-4 z-10 bg-white border rounded p-2 shadow-lg"
          >
            ☰
          </button>
        )}

        {/* Chat Window */}
        <div className="flex-1 flex flex-col max-w-4xl mx-auto w-full p-4">
          <div className="bg-white border rounded shadow flex-1 flex flex-col">
            <div className="flex justify-between items-center p-4 border-b">
              <h2 className="text-lg font-semibold">Chat with Llama</h2>
              <div className="flex items-center gap-2">
                {/* Server status indicator */}
                <div className="flex items-center text-xs">
                  <div className={`w-2 h-2 rounded-full mr-1 ${
                    !isLoading ? 'bg-green-500' : 'bg-yellow-500 animate-pulse'
                  }`}></div>
                  <span className="text-gray-600">
                    {!isLoading ? 'Ready' : 'Loading...'}
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
            <div className="p-4 border-b space-y-2">
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
            
            <div className="flex-1 overflow-y-auto p-4 bg-gray-50">
              {chatHistory.length === 0 ? (
                <p className="text-gray-400 text-center mt-8">Start the conversation...</p>
              ) : (
                chatHistory.map((msg, idx) => {
                  // For streaming messages, the text is already updated in real-time in the chat history
                  const isLastBotMessage = msg.sender === "bot" && idx === chatHistory.length - 1;
                  const displayText = msg.text;
                  const hasThinking = msg.thinking && msg.thinking.length > 0;
                  const isThinkingExpanded = expandedThinking[idx] || false;
                  
                  return (
                    <div key={idx} className={`mb-4 ${msg.sender === "user" ? "text-right" : "text-left"}`}>
                      {/* Thinking indicator for bot messages */}
                      {msg.sender === "bot" && hasThinking && (
                        <div className="mb-2">
                          <button
                            onClick={() => setExpandedThinking(prev => ({ ...prev, [idx]: !prev[idx] }))}
                            className="flex items-center gap-2 text-sm text-gray-600 hover:text-gray-800 bg-gray-100 rounded-lg px-3 py-2 border transition-colors"
                          >
                            <div className="flex items-center gap-1">
                              <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
                              <span>Thinking</span>
                            </div>
                            <span className="text-xs">
                              {isThinkingExpanded ? '▼' : '▶'}
                            </span>
                          </button>
                          {isThinkingExpanded && (
                            <div className="mt-2 bg-gray-50 border rounded-lg p-3 text-sm">
                              <div className="font-medium text-gray-700 mb-2">Model's thinking process:</div>
                              <div className="text-gray-600 whitespace-pre-wrap font-mono text-xs">
                                {msg.thinking}
                              </div>
                            </div>
                          )}
                        </div>
                      )}
                      
                      {/* Current thinking indicator for streaming */}
                      {isLastBotMessage && isThinking && thinkingEnabled && (
                        <div className="mb-2">
                          <div className="flex items-center gap-2 text-sm text-gray-600 bg-blue-50 rounded-lg px-3 py-2 border border-blue-200">
                            <div className="flex items-center gap-1">
                              <div className="w-2 h-2 bg-blue-500 rounded-full animate-pulse"></div>
                              <span>Thinking...</span>
                            </div>
                          </div>
                        </div>
                      )}
                      
                      <div className={`inline-block max-w-xs lg:max-w-md px-4 py-2 rounded-lg ${
                        msg.sender === "user" 
                          ? "bg-blue-600 text-white" 
                          : "bg-white border"
                      }`}>
                        <div className="font-semibold text-xs mb-1">
                          {msg.sender === "user" ? "You" : "Llama"}
                          {isLastBotMessage && isStreaming && (
                            <span className="ml-1 text-xs opacity-70">(streaming...)</span>
                          )}
                        </div>
                        <div className="whitespace-pre-wrap">
                          {displayText}
                          {isLastBotMessage && isStreaming && (
                            <span className="animate-pulse">▊</span>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
              <div ref={chatEndRef} />
            </div>
            
            {/* Conversation info */}
            {currentConversationId && (
              <div className="px-4 py-2 text-xs text-gray-500 border-t">
                Conversation ID: {currentConversationId.substring(0, 8)}...
              </div>
            )}
            
            <form className="p-4 border-t flex gap-2" onSubmit={handleSendChat}>
              <input
                className="flex-1 border rounded px-3 py-2"
                type="text"
                value={chatInput}
                onChange={e => setChatInput(e.target.value)}
                placeholder="Type your message..."
                autoFocus
              />
              <button
                className="bg-blue-600 text-white px-6 py-2 rounded hover:bg-blue-700 disabled:bg-gray-400"
                type="submit"
                disabled={!chatInput.trim() || isStreaming || isLoading}
              >
                {isStreaming ? 'Streaming...' : isLoading ? 'Loading...' : 'Send'}
              </button>
            </form>
          </div>
        </div>
      </div>
    </div>
  );
}
