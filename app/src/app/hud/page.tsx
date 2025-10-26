'use client';

import { useEffect, useRef, useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { HudDimensions } from '@/types/settings';
import { OcrResponseEvent } from '@/types/events';
import { useSettings } from '@/lib/settings';
import HUDInputBar from '@/components/hud/hud-input-bar';
import { useConversation } from '@/lib/conversations';
import { useWindows } from '@/lib/windows/useWindows';
import { DynamicChatContent } from '@/components/hud/dynamic-chat-content';

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

  // Refs
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
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
    dynamicChatContentRef,
    setChatExpanded,
    minimizeChat,
  } = useWindows();
  
  // Callback ref to sync both refs
  const dynamicChatContentCallback = useCallback((node: HTMLDivElement | null) => {
    dynamicChatContentRef.current = node;
  }, [dynamicChatContentRef]);

  // Load HUD dimensions
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const dimensions = await getHudDimensions();
      if (!cancelled) setHudDimensions(dimensions);
    })();
    return () => { cancelled = true; };
  }, [getHudDimensions]);
  
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

    const query = input.trim();

    if (!query || isLoading) return;

    await setChatExpanded();
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
  <div ref={containerRef} className="w-full h-full bg-blue-50a0">
      {/* Glass Container */}
      <div className="relative w-full h-full flex flex-col justify-start overflow-hidden">
        <div className="relative flex flex-col min-h-0 h-min">
          {/* Dynamic Chat Content Area */}
          <div ref={dynamicChatContentCallback}>
            <DynamicChatContent hudDimensions={hudDimensions} />
          </div>

          {/* Input Container - fixed height at bottom */}
          <HUDInputBar
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