'use client';

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { Move, X, MessageSquarePlus  } from 'lucide-react';
import Image from "next/image";
import Markdown from 'react-markdown'
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import { SettingsService } from '@/lib/settings-service';
import { HudDimensions } from '@/types/settings';
import { set } from 'react-hook-form';
const logo = '/logo.png';

interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export default function HudPage() {
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isExpanded, setIsExpanded] = useState(false);
  const isExpandedRef = useRef(false);

  // Keep ref in sync with state
  useEffect(() => {
    isExpandedRef.current = isExpanded;
  }, [isExpanded]);
  const [currentConversationId, setCurrentConversationId] = useState<string | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  const streamContentRef = useRef<string>('');
  const [messages, setMessages] = useState<{ role: 'user' | 'assistant'; content: string }[]>([]);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);

  const handleMouseLeave = async (e: React.MouseEvent) => {
    setIsHoveringGroup(false);
    // Get the bounding box of drag area
    const dragArea = document.getElementById('drag-area');
    if (!dragArea) return;

    const rect = dragArea.getBoundingClientRect();

    // Get the mouse coordinates in 100ms
    let mouseCoords = { x: e.clientX, y: e.clientY };

    // Print whether mouse is within bounding box
    const isWithinBox = rect && mouseCoords.x >= rect.left && mouseCoords.x <= rect.right && mouseCoords.y >= rect.top && mouseCoords.y <= rect.bottom;

    // set dragging off if not within bounding box
    if (!isWithinBox) {
      setIsDraggingWindow(false);
    }
  };

  // Ensure drag visibility resets when pointer is released anywhere
  useEffect(() => {
    const onUp = () => {setIsDraggingWindow(false); console.log('Pointer released');};
    window.addEventListener('pointerup', onUp);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('mouseup', onUp);
    };
  }, []);

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

    // print hud dimensions every time they change
  useEffect(() => {
      console.log('HUD dimensions changed:', hudDimensions);
    }, [hudDimensions]);

  // Initialize conversation, server, listener, and enable transparent background
  useEffect(() => {
    // Load HUD dimensions from settings
    const loadHudDimensions = async () => {
      try {
        const dimensions = await SettingsService.getHudDimensions();
        setHudDimensions(dimensions);
      } catch (error) {
        console.error('Failed to load HUD dimensions:', error);
        // Fallback to defaults
        const defaultDimensions: HudDimensions = {
          width: 500,
          collapsed_height: 60,
          expanded_height: 350,
        };
        setHudDimensions(defaultDimensions);
      }
    };

    loadHudDimensions();

    // Set up a listener for settings changes (we'll use a custom event)
    let unlistenSettings: UnlistenFn | null = null;
    (async () => {
      try {
        unlistenSettings = await listen('settings-changed', async () => {
          // Reload HUD dimensions when settings change
          await loadHudDimensions();
          
          // Also refresh the actual window size to match current state
          try {
            await invoke('refresh_hud_window_size', { 
              label: 'floating-hud', 
              isExpanded: isExpandedRef.current 
            });
          } catch (error) {
            console.error('Failed to refresh HUD window size:', error);
          }
        });
      } catch (err) {
        console.error('Failed to set up settings listener:', err);
      }
    })();

    // Ensure LLM server is running and create a conversation
    (async () => {
      try {
        // Try spawning the llama.cpp server (idempotent if already running)
        await invoke<string>('spawn_llama_server');
      } catch (e) {
        // Not fatal; generate() does its own health check too
        console.warn('spawn_llama_server failed or not available:', e);
      }
      const convId = await createNewConversation();
      // Load any existing messages for this conversation
      if (convId) {
        try {
          const existing = await invoke<any[]>('get_messages', { conversationId: convId });
          const mapped = existing.map((m) => ({ role: m.role === 'user' ? 'user' as const : 'assistant' as const, content: extractThinkingContent(m.content) }));
          setMessages(mapped);
        } catch (err) {
          console.error('Failed to load existing messages for HUD:', err);
        }
      }
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
            // Patch last assistant message
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) {
                next[lastIdx] = { ...next[lastIdx], content: finalText };
              }
              return next;
            });
            setIsLoading(false);
            setIsStreaming(false);
            streamContentRef.current = '';
            return;
          }

          if (delta) {
            // Accumulate then render cleaned content
            streamContentRef.current += delta;
            const clean = extractThinkingContent(streamContentRef.current);
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) {
                next[lastIdx] = { ...next[lastIdx], content: clean };
              } else {
                // If no assistant placeholder yet, add one
                next.push({ role: 'assistant', content: clean });
              }
              return next;
            });
            // Auto-scroll to bottom on new tokens
            queueMicrotask(() => {
              messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' });
            });
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
        try { unlistenStream(); } catch { }
      }
      if (unlistenSettings) {
        try { unlistenSettings(); } catch { }
      }
    };
  }, []);

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      console.log('Created conversation:', newConv.id);
      return newConv.id;
    } catch (err) {
      console.error("Error creating conversation:", err);
      return null;
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
      await invoke('resize_hud_expanded', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function collapseResponseArea() {
    setIsExpanded(false);
    try {
      await invoke('resize_hud_collapsed', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to resize window:', error);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const query = input.trim();

    if (!query || isLoading) return;

    setIsLoading(true);
    setInput('');

    await expandResponseArea();

    try {
      // Append user message and an assistant placeholder
      setMessages((prev) => [...prev, { role: 'user', content: query }, { role: 'assistant', content: '' }]);
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
      setMessages((prev) => {
        // If stream produced content, leave it. Otherwise set finalText.
        const next = [...prev];
        const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
        const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
        if (lastIdx >= 0) {
          const existing = next[lastIdx]?.content ?? '';
          next[lastIdx] = {
            role: 'assistant',
            content: existing && existing.length > 0 ? existing : extractThinkingContent(finalText),
          };
        }
        return next;
      });
      setIsStreaming(false);
      streamContentRef.current = '';

    } catch (error) {
      console.error('Error generating response:', error);
      setMessages((prev) => [...prev.slice(0, -1), { role: 'assistant', content: '[Error generating response]' }]);
      setIsLoading(false);
      setIsStreaming(false);
      streamContentRef.current = '';
    }
  }

  async function clearAndCollapse() {
    setMessages([]);
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
      {/* Glass Container */}
      <div className="relative w-full h-full backdrop-blur-2xl backdrop-saturate-150 flex flex-col overflow-hidden">

        {/* Chat Area - takes remaining space after input bar */}
        <div className="relative flex-1 flex flex-col min-h-0">
          {/* Messages Scroll Area */}
          {isExpanded && (
            <div
              ref={messagesContainerRef}
              className="hud-scroll flex-1 overflow-y-auto p-3 space-y-2 text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2 transition-all"
            >
              <div className="flex flex-col space-y-2">
                {messages.map((m, i) => (
                  <div
                    key={`m-${i}`}
                    className={
                      m.role === 'user'
                        ? 'max-w-[85%] ml-auto bg-white/60 border border-black/20 rounded-xl px-3 py-2'
                        : 'max-w-[95%] w-full text-left mx-auto'
                    }
                  >
                    {m.role === 'user' ?
                    <div className="whitespace-pre-wrap">{m.content}</div>
                    :
                    <Markdown {...llmMarkdownConfig}>
                      {m.content}
                    </Markdown>}
                  </div>
                ))}

                {isStreaming && (
                  <div className="inline mr-auto h-4 w-3 animate-pulse rounded-sm bg-black/30" />
                )}
                <div ref={messagesEndRef} />
              </div>
            </div>
          )}

          {/* Input Container - fixed height at bottom */}
          <div 
            className='flex-shrink-0 flex flex-col justify-center items-center relative p-2'
            id="input-container"
            onMouseEnter={() => setIsHoveringGroup(true)}
            onMouseLeave={handleMouseLeave}
            style={{
              height: hudDimensions ? `${hudDimensions.collapsed_height}px` : '60px',
              width: hudDimensions ? `${hudDimensions.width}px` : '500px'
            }}
          >
            <div
              className='flex items-center gap-3 rounded-lg bg-white/60 border border-black/20 transition-all focus-within:outline-none focus-within:ring-0 focus-within:border-black/20 flex-1 w-full'
            >
              <Image
                src={logo}
                width={32}
                height={32}
                alt="Logo"
                className="w-7 h-7 ml-2 select-none pointer-events-none"
                draggable={false}
                onDragStart={(e) => e.preventDefault()}
              />

              <div className="flex-1">
                <Input
                  type="text"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Ask anything"
                  className="bg-transparent rounded-none border-none shadow-none p-0 text-black placeholder:text-black/75 transition-all outline-none ring-0 focus:outline-none focus:ring-0 focus:ring-offset-0 focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0"
                  autoComplete="off"
                  autoFocus
                />
              </div>

              {/* New chat icon */}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className={`transform h-9 w-9 mr-5 rounded-full flex items-center justify-center hover:bg-white/60 transition-all duration-500 p-0 ${isExpanded ? "scale-100 opacity-100" : "scale-0 opacity-0"}`}
                    onClick={async () => {
                      await clearAndCollapse();
                      await createNewConversation();
                    }}
                    title="New Chat"
                  >
                    <MessageSquarePlus className="!w-5 !h-5 text-black shrink-0" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  <p>Start a new chat</p>
                </TooltipContent>
              </Tooltip>
            </div>
            
            {/* Close icon */}
            <button
              className={
              (isDraggingWindow || isHoveringGroup ? 'scale-100 opacity-100' : 'scale-0 opacity-0') +
              ' absolute top-0.5 right-0.5 w-6 h-6 rounded-full bg-white/60 hover:bg-white/80 border border-black/20 transition-all duration-100 select-none'
              }
              onClick={closeWindow}
              title="Close Window"
            >
              <X className="w-full h-full p-1 text-black pointer-events-none" />
            </button>

            {/* Move handle */}
            <div
              data-tauri-drag-region
              id="drag-area"
              className={
                (isDraggingWindow || isHoveringGroup ? 'scale-100 opacity-100' : 'scale-0 opacity-0') + 
                ' hover:cursor-grab select-none absolute bottom-0.5 right-0.5 w-6 h-6 bg-white/60 hover:bg-white/80 border border-black/20 rounded-full transition-all duration-100'
              }
              onPointerDown={() => setIsDraggingWindow(true)}
              draggable={false}
              title="Drag Window"
            >
                <Move className="w-full h-full p-1 text-black pointer-events-none" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}