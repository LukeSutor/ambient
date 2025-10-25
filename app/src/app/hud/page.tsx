'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { HudDimensions } from '@/types/settings';
import { OcrResponseEvent } from '@/types/events';
import { useSettings } from '@/lib/settings';
import MessageList from '@/components/hud/message-list';
import HUDInputBar from '@/components/hud/hud-input-bar';
import { useConversation } from '@/lib/conversations';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import { useWindows } from '@/lib/windows/useWindows';

export default function HudPage() {
  // UI State
  const [input, setInput] = useState('');
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  
  // OCR State
  const [ocrResults, setOcrResults] = useState<OcrResponseEvent[]>([]);
  const [ocrLoading, setOcrLoading] = useState(false);
  const ocrTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Window management cleanup function
  const [windowCleanup, setWindowCleanup] = useState<UnlistenFn | null>(null);

  // Refs
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const inputContainerRef = useRef<HTMLDivElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  // Conversation Manager
  const {
    messages,
    conversationId,
    isLoading,
    isStreaming,
    sendMessage,
    createNew,
    clear,
  } = useConversation(messagesEndRef);

  // Settings Manager
  const { getHudDimensions } = useSettings();

  // Window Manager
  const {
    isChatExpanded,
    messagesContainerRef,
    setExpandedChat,
    minimizeChat,
    trackContentAndResize,
  } = useWindows();
  
  // Keep a local ref for GSAP animations (needed for useHudAnimations)
  const localMessagesContainerRef = useRef<HTMLDivElement | null>(null);
  
  // Callback ref to sync both refs
  const messagesContainerCallback = useCallback((node: HTMLDivElement | null) => {
    localMessagesContainerRef.current = node;
    messagesContainerRef.current = node;
  }, [messagesContainerRef]);

  // Load HUD dimensions
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const dimensions = await getHudDimensions();
      if (!cancelled) setHudDimensions(dimensions);
    })();
    return () => { cancelled = true; };
  }, [getHudDimensions]);
  
  useGSAP(() => {
    if (hudDimensions && inputContainerRef.current) {
      gsap.fromTo(
        inputContainerRef.current,
        { scale: 0, opacity: 0, transformOrigin: 'center center' },
        { scale: 1, opacity: 1, duration: 0.25, ease: 'back.out(0.8)', delay: 0.1 }
      );
    }
  }, [hudDimensions]);

  // Set up OCR listener and initialize HUD size after dimensions are loaded
  useEffect(() => {
    // Only initialize if dimensions are loaded
    if (!hudDimensions) return;

    const setupOcrListener = async () => {
      try {
        const unlisten = await listen<OcrResponseEvent>('ocr_response', (event) => {
          const result = event.payload;
          if (!result.success) console.error('OCR failed');
          
          if (ocrTimeoutRef.current) {
            clearTimeout(ocrTimeoutRef.current);
            ocrTimeoutRef.current = null;
          }
          
          setOcrResults((prev) => [...prev, result]);
          setOcrLoading(false);
        });
        return unlisten;
      } catch (error) {
        console.error('Failed to set up OCR listener:', error);
        return null;
      }
    };

    let cleanup: UnlistenFn | null = null;
    setupOcrListener().then((fn) => {
      cleanup = fn;
    });

    return () => {
      if (cleanup) {
        try {
          cleanup();
        } catch (error) {
          console.error('Error cleaning up OCR listener:', error);
        }
      }
      if (ocrTimeoutRef.current) {
        clearTimeout(ocrTimeoutRef.current);
        ocrTimeoutRef.current = null;
      }
    };
  }, [hudDimensions, minimizeChat]);

  // Track content height changes and resize window dynamically during streaming
  useEffect(() => {
    const cleanup = trackContentAndResize();
    setWindowCleanup(() => cleanup);
    return cleanup;
  }, [trackContentAndResize]);

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
    const onUp = () => setIsDraggingWindow(false);
    window.addEventListener('pointerup', onUp);
    window.addEventListener('mouseup', onUp);
    return () => {
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('mouseup', onUp);
    };
  }, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();

    if (!windowCleanup) {
      const cleanup = trackContentAndResize();
      setWindowCleanup(() => cleanup);
    }

    const query = input.trim();

    if (!query || isLoading) return;

    await setExpandedChat();
    setInput('');

    // Ensure we have a conversation
    let convId = conversationId;
    if (!convId) {
      convId = await createNew();
      if (!convId) {
        console.error('Failed to create conversation');
        return;
      }
    }

    try {
      // Send message (will optimistically update UI)
      await sendMessage(convId, query, ocrResults);
      
      // Clear OCR results after sending
      setOcrResults([]);
    } catch (error) {
      console.error('Error in handleSubmit:', error);
    }
  }

  async function clearAndCollapse() {
    if (windowCleanup) {
      windowCleanup();
      setWindowCleanup(null);
    }
    clear(250);
    await minimizeChat(300);
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e as any);
    }
    if (e.key === 'Escape') {
      handleNewChat();
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
      await createNew();
    }
  }

  return (
  <div ref={containerRef} className="w-full h-full bg-blue-500">
      {/* Glass Container */}
      <div className="relative w-full h-full flex flex-col justify-start overflow-hidden">

        {/* Chat Area - takes remaining space after input bar */}
        <div className="relative flex flex-col min-h-0 h-min">
          {/* Messages Scroll Area */}
            <div
            ref={messagesContainerCallback}
            className="hud-scroll h-full overflow-y-auto space-y-2 text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2"
            style={{maxHeight: hudDimensions?.chat_max_height ?? 500}}
            >
            <MessageList ref={messagesEndRef} />
            </div>

          {/* Input Container - fixed height at bottom */}
          <HUDInputBar
            ref={inputContainerRef}
            hudDimensions={hudDimensions}
            inputValue={input}
            setInputValue={setInput}
            onKeyDown={handleKeyDown}
            onCaptureArea={handleCaptureArea}
            onNewChat={handleNewChat}
            onDragStart={() => setIsDraggingWindow(true)}
            onMouseLeave={handleMouseLeave}
            isDraggingWindow={isDraggingWindow}
            isHoveringGroup={isHoveringGroup}
            setIsHoveringGroup={setIsHoveringGroup}
            ocrLoading={ocrLoading}
            ocrResults={ocrResults}
            removeOcrAt={(i) => setOcrResults((prev) => prev.filter((_, idx) => idx !== i))}
            isStreaming={isStreaming}
          />
        </div>
      </div>
    </div>
  );
}