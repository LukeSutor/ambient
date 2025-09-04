'use client';

import { useState, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Separator } from "@/components/ui/separator"
import { Move, X, LoaderCircle, MessageSquarePlus, Plus, SquareDashedMousePointer, SquareDashed } from 'lucide-react';
import Image from "next/image";
import Markdown from 'react-markdown'
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import { SettingsService } from '@/lib/settings-service';
import { HudDimensions } from '@/types/settings';
import { OcrResponseEvent, HudChatEvent, ChatStreamEvent } from "@/types/events";
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';

// Simple text component without animation
const AnimatedText = ({ content }: { content: string }) => {
  return <div className="whitespace-pre-wrap">{content}</div>;
};

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
  const currentConversationIdRef = useRef<string | null>(null);
  useEffect(() => {
    currentConversationIdRef.current = currentConversationId;
  }, [currentConversationId]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  const streamContentRef = useRef<string>('');
  const [messages, setMessages] = useState<{ role: 'user' | 'assistant'; content: string }[]>([]);
  const [plusExpanded, setPlusExpanded] = useState(false);
  const [ocrResults, setOcrResults] = useState<OcrResponseEvent[]>([]);
  const [ocrLoading, setOcrLoading] = useState(false);
  const ocrTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);
  const inputContainerRef = useRef<HTMLDivElement | null>(null);
  const toolboxDropdownRef = useRef<HTMLDivElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const measurementRef = useRef<HTMLDivElement>(null);

  // GSAP animation for input box spring entrance
  useGSAP(() => {
    if (hudDimensions && inputContainerRef.current) {
      // Animate the input box from invisible to visible with spring effect
      gsap.fromTo(inputContainerRef.current, 
        {
          scale: 0,
          opacity: 0,
          transformOrigin: "center center"
        },
        {
          scale: 1,
          opacity: 1,
          duration: 0.25,
          ease: "back.out(0.8)",
          delay: 0.05
        }
      );
    }
  }, [hudDimensions]); // Re-run animation when hudDimensions loads

  // GSAP animation for chat expansion when first message is sent
  useGSAP(() => {
    if (isExpanded && messagesContainerRef.current && messages.length > 0) {      
      // Create a timeline for coordinated animations
      const tl = gsap.timeline();
      
      // Instantly apply padding and hide overflow at the start
      gsap.set(messagesContainerRef.current, { 
        padding: "12px",
        overflowY: "hidden" 
      });
      
      // Then animate the messages container growing in
      tl.to(messagesContainerRef.current,
        {
          height: "auto",
          opacity: 1,
          scale: 1,
          duration: 0.6,
          ease: "back.out(1.2)",
          onComplete: () => {
            // Restore scrolling after animation completes
            if (messagesContainerRef.current) {
              gsap.set(messagesContainerRef.current, { overflowY: "auto" });
            }
          }
        }
      );
      
      // Simultaneously animate the input container moving down (if needed)
      if (inputContainerRef.current) {
        tl.to(inputContainerRef.current,
          {
            y: 0, // Ensure it's in final position
            duration: 0.6,
            ease: "back.out(1.2)"
          },
          0 // Start at the same time as the messages animation
        );
      }
    } else if (!isExpanded && messagesContainerRef.current) {
      // Animate collapse - reset to initial state
      gsap.to(messagesContainerRef.current, {
        height: 0,
        opacity: 0,
        scale: 0.95,
        padding: "0px",
        overflowY: "hidden", // Hide overflow during collapse too
        duration: 0.4,
        ease: "power2.inOut"
      });
    }
  }, [isExpanded, messages.length]); // Re-run when expansion state or message count changes

  // Smooth height animation for messages container when content changes
  useEffect(() => {
    if (!messagesContainerRef.current || !isExpanded || messages.length === 0) return;

    let animationFrame: number;
    let lastHeight = 0;

    const checkHeightChange = () => {
      const container = messagesContainerRef.current;
      if (!container) return;

      const contentDiv = container.querySelector('.flex.flex-col.space-y-2') as HTMLElement;
      if (!contentDiv) return;

      const newHeight = contentDiv.scrollHeight;
      
      if (newHeight !== lastHeight && lastHeight > 0 && isStreaming) {
        // Smoothly animate height change during streaming
        gsap.to(container, {
          height: newHeight,
          duration: 0.25,
          ease: "power2.out"
        });
      }
      
      lastHeight = newHeight;
      
      if (isStreaming) {
        animationFrame = requestAnimationFrame(checkHeightChange);
      }
    };

    if (isStreaming) {
      // Start monitoring height changes during streaming
      const container = messagesContainerRef.current;
      const contentDiv = container.querySelector('.flex.flex-col.space-y-2') as HTMLElement;
      if (contentDiv) {
        lastHeight = contentDiv.scrollHeight;
        animationFrame = requestAnimationFrame(checkHeightChange);
      }
    }

    return () => {
      if (animationFrame) {
        cancelAnimationFrame(animationFrame);
      }
    };
  }, [isExpanded, messages.length, isStreaming]);

  // Add ResizeObserver to containerRef to detect height changes
  useEffect(() => {
    if (!containerRef.current) return;
    const resizeObserver = new ResizeObserver(async (entries) => {
      for (let entry of entries) {
        if (entry.contentRect) {
          const newHeight = entry.contentRect.height;
          try {
            await invoke('resize_hud_dynamic', { label: 'floating-hud', additionalHeight: newHeight });
          } catch (error) {
            console.error('Failed to update HUD window height:', error);
          }
        }
      }
    });
    // if (measurementRef.current) {
    //   resizeObserver.observe(measurementRef.current);
    // }
    return () => {
      resizeObserver.disconnect();
    };
  }, [measurementRef]);

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
    clearAndCollapse();
    const onUp = () => setIsDraggingWindow(false);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('mouseup', onUp);
    // async function resizeHud() {
    //   try {
    //     await invoke('resize_hud_expanded', { label: 'floating-hud' });
    //   } catch (error) {
    //     console.error('Failed to resize window:', error);
    //   }
    // };
    // resizeHud();
    return () => {
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('mouseup', onUp);
    };
  }, []);

  // Helper to strip <think> blocks
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

  // Initialize conversation, server, chat listener, ocr listener, and enable transparent background
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
        unlistenSettings = await listen('settings_changed', async () => {
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
        unlistenStream = await listen<ChatStreamEvent>('chat_stream', (event) => {
          const { delta, full_response, is_finished, conv_id } = event.payload;

          // Ignore if not from current conversation
          if (conv_id !== currentConversationIdRef.current) {
            console.log('Ignoring stream event from different conversation');
            return;
          }

          if (is_finished) {
            const finalText = extractThinkingContent(full_response ?? streamContentRef.current);
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
        console.error('Failed to set up chat_stream listener:', err);
      }
    })();

    // Listen for ocr events
    let unlistenOCR: UnlistenFn | null = null;
    (async () => {
      try {
        unlistenOCR = await listen<OcrResponseEvent>('ocr_response', (event) => {
          const result = event.payload as OcrResponseEvent;
          if (!result.success) {
            console.error('OCR failed');
          }
          // Clear any active timeout as we've received a result
          if (ocrTimeoutRef.current) {
            clearTimeout(ocrTimeoutRef.current);
            ocrTimeoutRef.current = null;
          }
          setOcrResults((prev) => [...prev, result]);
          setOcrLoading(false);
        });
      } catch (err) {
        console.error('Failed to set up OCR listener:', err);
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
      if (unlistenOCR) {
        try { unlistenOCR(); } catch { }
      }
      // Clear any pending OCR timeout on unmount
      if (ocrTimeoutRef.current) {
        clearTimeout(ocrTimeoutRef.current);
        ocrTimeoutRef.current = null;
      }
    };
  }, []);

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>("create_conversation", { name: null });
      setCurrentConversationId(newConv.id);
      currentConversationIdRef.current = newConv.id;
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

    // Fully expand window to allow for animations
    try {
      await invoke('resize_hud_expanded', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to resize window:', error);
    }

    setIsLoading(true);
    setIsExpanded(true);
    setInput('');

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

      // Create hud chat event
      const hudChatEvent: HudChatEvent = {
        text: query,
        ocr_responses: ocrResults,
        conv_id: convId,
        timestamp: Date.now().toString(),
      };

      // Generate text with custom hud chat function
      const finalText = await invoke<string>('handle_hud_chat', {
        event: hudChatEvent
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

      // Dynamically set the size to the content height
      if (messagesContainerRef.current) {
        const scrollHeight = messagesContainerRef.current.scrollHeight;
        try {
          await invoke('resize_hud_dynamic', { label: 'floating-hud', additionalHeight: scrollHeight });
        } catch (error) {
          console.error('Failed to resize window:', error);
        }
      }

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

  const handleCaptureArea = async () => {
    // Reset any previous timeout and start loading
    if (ocrTimeoutRef.current) {
      clearTimeout(ocrTimeoutRef.current);
      ocrTimeoutRef.current = null;
    }
    setOcrLoading(true);
    try {
      await invoke('open_screen_selector');
      // Start a 10s timeout; if no OCR result arrives, stop loading
      ocrTimeoutRef.current = setTimeout(() => {
        console.warn('OCR capture timed out after 10s.');
        setOcrLoading(false);
        ocrTimeoutRef.current = null;
      }, 10000);
    } catch (error: any) {
      console.error('Failed to open screen selector:', error);
      setOcrLoading(false);
      if (ocrTimeoutRef.current) {
        clearTimeout(ocrTimeoutRef.current);
        ocrTimeoutRef.current = null;
      }
    }
    setPlusExpanded(false);
  };

  const handleNewChat = async () => {
    // Don't create new conversation if there are no messages
    if (messages.length > 0) {
      await clearAndCollapse();
      await createNewConversation();
    }
    setPlusExpanded(false);
  }

  const handleLogoClick = async () => {
    try {
      await invoke('open_main_window');
    } catch (error) {
      console.error('Failed to open main window:', error);
    }
  }

  const handleExpandFeatures = async () => {
    // Resize to fit if expanding
    if (!plusExpanded) {
      try {
        await invoke('resize_hud_dynamic', { label: 'floating-hud', additionalHeight: 64 });
      } catch (error) {
        console.error('Failed to update HUD window height:', error);
      }
      setPlusExpanded(true);
    } else {
      try {
        await invoke('resize_hud_collapsed', { label: 'floating-hud' });
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
      setPlusExpanded(false);
    }
  }

  return (
    <div ref={containerRef} className="w-full h-full bg-blue-500">
      {/* Hidden measurement container - exactly mirrors the real messages container */}
      <div
        ref={measurementRef}
        className="absolute opacity-0 pointer-events-none"
        style={{
          width: hudDimensions?.width ? `${hudDimensions.width}px` : '500px',
          top: '-9999px' // Move offscreen
        }}
      >
        <div className="hud-scroll flex-1 overflow-y-auto p-3 space-y-2 text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2 transition-all">
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
            <div />
          </div>
        </div>
      </div>

      {/* Glass Container */}
      <div className="relative w-full h-full flex flex-col justify-start overflow-hidden">

        {/* Chat Area - takes remaining space after input bar */}
        <div className="relative flex flex-col min-h-0 h-min">
          {/* Messages Scroll Area */}
          <div ref={messagesContainerRef}
            className="hud-scroll overflow-y-auto space-y-2 text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2"
            style={{
              height: '0px',
              opacity: 0,
              transform: 'scale(0.95)',
              transformOrigin: 'center bottom',
              padding: '0px'
            }}
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
                    <div className="prose prose-sm max-w-none">
                      <AnimatedText content={m.content} />
                    </div>}
                </div>
              ))}
              <div ref={messagesEndRef} />
            </div>
          </div>

          {/* Input Container - fixed height at bottom */}
          <div
            ref={inputContainerRef}
            className='flex-shrink-0 flex flex-col justify-center items-center relative p-2'
            id="input-container"
            onMouseEnter={() => setIsHoveringGroup(true)}
            onMouseLeave={handleMouseLeave}
            style={{
              height: hudDimensions ? `${hudDimensions.collapsed_height}px` : '60px',
              width: hudDimensions ? `${hudDimensions.width}px` : '500px',
              opacity: hudDimensions ? 1 : 0,
              transform: hudDimensions ? 'scale(1)' : 'scale(0)'
            }}
          >
            <div
              className='flex items-center gap-3 rounded-lg bg-white/60 border border-black/20 transition-all focus-within:outline-none focus-within:ring-0 focus-within:border-black/20 flex-1 w-full'
            >
              <button onClick={handleLogoClick} title="Open Main Window" className="shrink-0">
                <Image
                  src={logo}
                  width={32}
                  height={32}
                  alt="Logo"
                  className="w-7 h-7 ml-2 select-none pointer-events-none shrink-0"
                  draggable={false}
                  onDragStart={(e) => e.preventDefault()}
                />
              </button>

              <div className="flex-1 min-w-32">
                <Input
                  type="text"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  onKeyDown={handleKeyDown}
                  placeholder="Ask anything"
                  className="bg-transparent rounded-none border-none shadow-none p-0 text-black placeholder:text-black/75 transition-all outline-none ring-0 focus:outline-none focus:ring-0 focus:ring-offset-0 focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0 min-w-0 w-full"
                  autoComplete="off"
                  autoFocus
                />
              </div>

              {/* OCR captures */}
              <div className="flex items-center gap-1 overflow-hidden whitespace-nowrap shrink min-w-0">
                {ocrResults.map((capture, index) => (
                  <div
                    key={index}
                    className="flex items-center justify-center bg-blue-500/30 rounded-xl px-2 py-1 shrink-0"
                    title={capture.text.length > 15 ? capture.text.slice(0, 15) + '...' : capture.text}
                  >
                    <SquareDashed className="!h-5 !w-5" />
                    <Button
                      variant="ghost"
                      className="!h-5 !w-5 text-black shrink-0 hover:bg-transparent"
                      size="icon"
                      onClick={() => {
                        setOcrResults(prev => prev.filter((_, i) => i !== index));
                      }}
                    >
                      <X className="!h-3 !w-3 text-black shrink-0" />
                    </Button>
                  </div>
                ))}
              </div>

              {/* Additional features expandable area */}
              <div className={`relative flex flex-row justify-end items-center w-auto min-w-8 h-8 rounded-full hover:bg-white/60 mr-5 transition-all ${plusExpanded ? "bg-white/40" : ""} shrink-0`} ref={toolboxDropdownRef}>
                <div className={`absolute bottom-full mb-1 right-0 bg-white/40 border border-black/20 rounded-lg p-2 flex flex-col gap-2 transition-all duration-300 ease-in-out overflow-hidden ${plusExpanded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2 pointer-events-none'}`}>
                  <Button
                    variant="ghost"
                    className="flex items-center gap-2 h-8 px-3 rounded-md hover:bg-white/60 justify-start"
                    onClick={handleCaptureArea}
                    title="Capture Area"
                  >
                    <SquareDashedMousePointer className="!w-4 !h-4 text-black shrink-0" />
                    <span className="text-black text-sm whitespace-nowrap">Capture Area</span>
                  </Button>
                  <Button
                    variant="ghost"
                    className="flex items-center gap-2 h-8 px-3 rounded-md hover:bg-white/60 justify-start"
                    onClick={handleNewChat}
                    title="New Chat"
                  >
                    <MessageSquarePlus className="!w-4 !h-4 text-black shrink-0" />
                    <span className="text-black text-sm whitespace-nowrap">New Chat</span>
                  </Button>
                </div>
                <Button
                  variant="ghost"
                  className="w-8 h-8 rounded-full"
                  size="icon"
                  disabled={ocrLoading}
                  onClick={handleExpandFeatures}
                >
                  {ocrLoading ? <LoaderCircle className="!h-5 !w-5 animate-spin" /> : <Plus className={`!h-5 !w-5 text-black shrink-0 transition-transform duration-300 ${plusExpanded ? 'rotate-45' : 'rotate-0'}`} />}
                </Button>
              </div>
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