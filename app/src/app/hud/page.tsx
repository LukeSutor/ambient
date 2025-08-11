'use client';

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { LogicalSize } from '@tauri-apps/api/dpi';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Move, X } from 'lucide-react';

interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export default function HudPage() {
  const [input, setInput] = useState('');
  const [response, setResponse] = useState('');
  const [userQuery, setUserQuery] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const streamContentRef = useRef<string>('');

  // Helper to strip <think> blocks similar to landing page
  function extractThinkingContent(text: string) {
    const thinkStartIndex = text.indexOf('<think>');
    const thinkEndIndex = text.indexOf('</think>');
    let cleanText = text;
    if (thinkStartIndex !== -1) {
      if (thinkEndIndex !== -1) {
        cleanText = text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
      } else {
        cleanText = text.substring(0, thinkStartIndex);
      }
    }
    return cleanText;
  }

  // Initialize conversation, server, listener, and enable transparent background
  useEffect(() => {
    // Ensure LLM server is running and create a conversation
    (async () => {
      try {
        // Try spawning the llama.cpp server (idempotent if already running)
        await invoke<string>('spawn_llama_server');
      } catch (e) {
        // Not fatal; generate() does its own health check too
        console.warn('spawn_llama_server failed or not available:', e);
      }
      await createNewConversation();
    })();

    // Set up a single stream listener mirroring the main UI approach
    let unlistenStream: UnlistenFn | null = null;
    (async () => {
      try {
        unlistenStream = await listen('chat-stream', (event) => {
          const payload: any = event.payload as any;
          const delta: string = payload?.delta ?? '';
          const isFinished: boolean = Boolean(payload?.is_finished);
          const full: string | undefined = payload?.full_response;

          if (isFinished) {
            const finalText = extractThinkingContent(full ?? streamContentRef.current);
            setResponse(finalText);
            setIsLoading(false);
            setIsStreaming(false);
            streamContentRef.current = '';
            return;
          }

          if (delta) {
            // Accumulate then render cleaned content
            streamContentRef.current += delta;
            const clean = extractThinkingContent(streamContentRef.current);
            setResponse(clean);
          }
        });
      } catch (err) {
        console.error('Failed to set up chat-stream listener:', err);
      }
    })();

    // Add class to force transparent bg for this window
    if (typeof document !== 'undefined') {
      document.documentElement.classList.add('hud-transparent');
      document.body.classList.add('hud-transparent');
    }

    return () => {
      if (typeof document !== 'undefined') {
        document.documentElement.classList.remove('hud-transparent');
        document.body.classList.remove('hud-transparent');
      }
      if (unlistenStream) {
        try { unlistenStream(); } catch {}
      }
    };
  }, []);

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      console.log('Created conversation:', newConv.id);
    } catch (err) {
      console.error("Error creating conversation:", err);
    }
  }

  async function closeWindow() {
    try {
      await invoke('close_floating_window', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to close window:', error);
      try {
        const currentWindow = getCurrentWebviewWindow();
        await currentWindow.close();
      } catch (altError) {
        console.error('Direct close method also failed:', altError);
      }
    }
  }

  async function expandResponseArea() {
    setIsExpanded(true);
    try {
      const currentWindow = getCurrentWebviewWindow();
      const newSize = new LogicalSize(400, 350);
      await currentWindow.setSize(newSize);
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function collapseResponseArea() {
    setIsExpanded(false);
    try {
      const currentWindow = getCurrentWebviewWindow();
      const newSize = new LogicalSize(400, 80);
      await currentWindow.setSize(newSize);
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const query = input.trim();
    
    if (!query || isLoading) return;
    
    setIsLoading(true);
    setUserQuery(query);
    setInput('');
    
    await expandResponseArea();
    
  try {
      setResponse('');
      setIsStreaming(true);
      streamContentRef.current = '';

      // Ensure we have a conversation id, create directly if missing to avoid race with state update
      let convId = currentConversationId;
      if (!convId) {
        try {
          const newConv = await invoke<Conversation>('create_conversation', { name: null });
          setCurrentConversationId(newConv.id);
          convId = newConv.id;
        } catch (convErr) {
          console.error('Failed to create conversation:', convErr);
        }
      }

      // Kick off generation with streaming always on and thinking disabled
      const finalText = await invoke<string>('generate', {
        prompt: query,
        jsonSchema: null,
        convId: convId ?? null,
        useThinking: false,
        stream: true,
      });

      // Safety: if no stream events arrive, stop loading when invoke resolves
      setIsLoading(false);
      setResponse((prev) => (prev && prev.length > 0 ? prev : extractThinkingContent(finalText)));
      setIsStreaming(false);
      streamContentRef.current = '';
      
    } catch (error) {
      console.error('Error generating response:', error);
      setResponse('Sorry, there was an error processing your request.');
      setIsLoading(false);
      setIsStreaming(false);
      streamContentRef.current = '';
    }
  }

  async function clearAndCollapse() {
    setUserQuery('');
    setResponse('');
    await collapseResponseArea();
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e as any);
    }
    if (e.key === 'Escape') {
      clearAndCollapse();
    }
  };

  return (
    <div className="w-full h-full bg-transparent">
      <div className="w-full h-full bg-transparent">
        {/* Glass Container */}
        <div className="relative w-full h-full rounded-2xl bg-white/40 backdrop-blur-2xl backdrop-saturate-150 ring-1 ring-black/10 shadow-[0_8px_30px_rgba(0,0,0,0.08)] flex flex-col overflow-hidden">
          {/* Subtle sheen and vignette overlays for depth (light glass) */}
          <div className="pointer-events-none absolute inset-0 bg-gradient-to-b from-white/50 via-transparent to-transparent" />
          <div className="pointer-events-none absolute inset-0 [mask-image:radial-gradient(80%_80%_at_50%_30%,black,transparent)] shadow-[inset_0_-40px_80px_rgba(0,0,0,0.10)]" />
        
        {/* Close Button */}
        <Button
          variant="ghost"
          size="sm"
          onClick={closeWindow}
          className="absolute top-2 right-2 w-6 h-6 p-0 rounded-full bg-black/5 border border-black/10 text-black/70 hover:bg-black/10 hover:border-black/20 hover:scale-110 transition-all z-10"
        >
          <X className="w-3 h-3" />
        </Button>

        {/* Input Container */}
        <div className="flex items-center p-3 gap-3 flex-shrink-0 bg-white/30 border-b border-black/10">
          <div className="flex-1">
            <Input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask me anything..."
              className="bg-white/60 border-black/15 rounded-xl text-black placeholder:text-black/50 focus:bg-white/70 focus:border-black/30 focus:shadow-[0_0_0_1px_rgba(0,0,0,0.08),0_0_20px_rgba(0,0,0,0.08)] backdrop-blur-sm transition-all"
              autoComplete="off"
              autoFocus
            />
          </div>
          
          {/* Drag Handle */}
          <div
            data-tauri-drag-region
            className="w-9 h-9 rounded-full bg-black/5 border border-black/10 flex items-center justify-center cursor-grab hover:bg-black/10 hover:border-black/20 hover:scale-105 active:scale-95 backdrop-blur-sm transition-all"
          >
            <Move className="w-4 h-4 text-black/70" />
          </div>
        </div>

        {/* Response Container */}
        {isExpanded && (
          <div className="flex-1 overflow-y-auto">
            <div className="p-4 text-black/90 text-sm leading-relaxed bg-white/30">
              {userQuery && (
                <div className="bg-white/60 border border-black/10 rounded-lg p-3 mb-3 font-medium text-black">
                  {userQuery}
                </div>
              )}
              
              <div className="whitespace-pre-wrap">
                {isLoading ? (
                  <div className="flex items-center gap-1">
                    <div className="flex gap-1">
                      <div className="w-1 h-1 bg-black/60 rounded-full animate-bounce [animation-delay:-0.32s]"></div>
                      <div className="w-1 h-1 bg-black/60 rounded-full animate-bounce [animation-delay:-0.16s]"></div>
                      <div className="w-1 h-1 bg-black/60 rounded-full animate-bounce"></div>
                    </div>
                  </div>
                ) : (
                  response
                )}
              </div>
            </div>
          </div>
        )}
        </div>
      </div>
    </div>
  );
}