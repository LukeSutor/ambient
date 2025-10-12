'use client';

import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { HudDimensions } from '@/types/settings';
import { ChatStreamEvent, HudChatEvent, OcrResponseEvent } from '@/types/events';
import { MemoryEntry } from '@/types/memory';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import MessageList from '@/components/hud/message-list';
import HUDInputBar from '@/components/hud/hud-input-bar';
import { useHudAnimations } from '@/hooks/use-hud-animations';
import { useHudStream } from '@/hooks/use-hud-stream';

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
  const hudDimensionsRef = useRef<HudDimensions | null>(null);
  useEffect(() => {
    hudDimensionsRef.current = hudDimensions;
  }, [hudDimensions]);
  const streamContentRef = useRef<string>('');
  const [messages, setMessages] = useState<{ role: 'user' | 'assistant'; content: string; memory: MemoryEntry | null }[]>([]);
  const [plusExpanded, setPlusExpanded] = useState(false);
  const [ocrResults, setOcrResults] = useState<OcrResponseEvent[]>([]);
  const [ocrLoading, setOcrLoading] = useState(false);
  const ocrTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);
  const inputContainerRef = useRef<HTMLDivElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const measurementRef = useRef<HTMLDivElement>(null);

  // Encapsulated GSAP animations
  useHudAnimations({
    hudDimensions,
    inputContainerRef,
    messagesContainerRef,
    isExpanded,
    messagesLength: messages.length,
    isStreaming,
  });

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

  // Stream/events setup
  const { createNewConversation, closeWindow } = useHudStream({
    isExpandedRef,
    setHudDimensions,
    hudDimensionsRef,
    setMessages,
    setIsLoading,
    setIsStreaming,
    setOcrResults,
    setOcrLoading,
    ocrTimeoutRef,
    currentConversationIdRef,
    setCurrentConversationId,
    messagesEndRef,
  });

  // createNewConversation and closeWindow provided by useHudStream

  async function collapseResponseArea() {
    setIsExpanded(false);
    // Collapse HUD after 250ms to allow input box animation
    setTimeout(async () => {
      try {
        await invoke('resize_hud_collapsed', { label: 'floating-hud' });
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    }, 500);
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
      setMessages((prev) => [...prev, { role: 'user', content: query, memory: null }, { role: 'assistant', content: '', memory: null }]);
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

      // Throw error if still no conversation id
      if (!convId) throw new Error('No conversation ID available');

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
            memory: null,
          };
        }
        return next;
      });
    } catch (error) {
      console.error('Error generating response:', error);
      setMessages((prev) => [...prev.slice(0, -1), { role: 'assistant', content: '[Error generating response]', memory: null }]);
      setIsLoading(false);
    } finally {
      streamContentRef.current = '';
      setIsStreaming(false);
      // Dynamically set the size to the content height
      if (messagesContainerRef.current) {
        const scrollHeight = messagesContainerRef.current.scrollHeight;
        try {
          await invoke('resize_hud_dynamic', { label: 'floating-hud', additionalHeight: scrollHeight });
        } catch (error) {
          console.error('Failed to resize window:', error);
        }
      }
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
  };

  const handleNewChat = async () => {
    // Don't create new conversation if there are no messages
    if (messages.length > 0) {
      await clearAndCollapse();
      await createNewConversation();
    }
  }

  const handleLogoClick = async () => {
    try {
      await invoke('open_main_window');
    } catch (error) {
      console.error('Failed to open main window:', error);
    }
  }

  const handleExpandFeatures = async () => {
    // Resize to fit if expanding and no messages
    if (!plusExpanded) {
      if (messages.length === 0) {
        try {
          await invoke('resize_hud_dynamic', { additionalHeight: 76 });
        } catch (error) {
          console.error('Failed to update HUD window height:', error);
        }
      }
      setPlusExpanded(true);
    // Only collapse if no messages
    } else {
      if (messages.length === 0) {
        // Collapse after a quarter second to allow button press animation
        setTimeout(async () => {
          try {
            await invoke('resize_hud_collapsed', { label: 'floating-hud' });
          } catch (error) {
            console.error('Failed to resize window:', error);
          }
        }, 250);
      }
      setPlusExpanded(false);
    }
  }

  return (
  <div ref={containerRef} className="w-full h-full bg-transparent">
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
          <MessageList messages={messages} showMarkdown />
        </div>
      </div>

      {/* Glass Container */}
      <div className="relative w-full h-full flex flex-col justify-start overflow-hidden">

        {/* Chat Area - takes remaining space after input bar */}
        <div className="relative flex flex-col min-h-0 h-min">
          {/* Messages Scroll Area */}
          <div
            ref={messagesContainerRef}
            className="hud-scroll overflow-y-auto space-y-2 text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2"
            style={{ height: '0px', opacity: 0, transform: 'scale(0.95)', transformOrigin: 'center bottom', padding: '0px' }}
          >
            <MessageList ref={messagesEndRef} messages={messages} showMarkdown={false} />
          </div>

          {/* Input Container - fixed height at bottom */}
          <HUDInputBar
            ref={inputContainerRef}
            hudDimensions={hudDimensions}
            inputValue={input}
            setInputValue={setInput}
            onKeyDown={handleKeyDown}
            onLogoClick={handleLogoClick}
            onExpandFeatures={handleExpandFeatures}
            onCaptureArea={handleCaptureArea}
            onNewChat={handleNewChat}
            onClose={closeWindow}
            onDragStart={() => setIsDraggingWindow(true)}
            onMouseLeave={handleMouseLeave}
            isDraggingWindow={isDraggingWindow}
            isHoveringGroup={isHoveringGroup}
            setIsHoveringGroup={setIsHoveringGroup}
            plusExpanded={plusExpanded}
            setPlusExpanded={setPlusExpanded}
            ocrLoading={ocrLoading}
            ocrResults={ocrResults}
            removeOcrAt={(i) => setOcrResults((prev) => prev.filter((_, idx) => idx !== i))}
            messagesCount={messages.length}
          />
        </div>
      </div>
    </div>
  );
}