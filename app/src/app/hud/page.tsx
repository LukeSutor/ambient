'use client';

import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { HudDimensions } from '@/types/settings';
import { OcrResponseEvent } from '@/types/events';
import { SettingsService } from '@/lib/settings-service';
import MessageList from '@/components/hud/message-list';
import HUDInputBar from '@/components/hud/hud-input-bar';
import { useHudAnimations } from '@/hooks/use-hud-animations';
import { useConversationManager } from '@/lib/conversations';

export default function HudPage() {
  // UI State
  const [input, setInput] = useState('');
  const [isExpanded, setIsExpanded] = useState(false);
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [plusExpanded, setPlusExpanded] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  
  // OCR State
  const [ocrResults, setOcrResults] = useState<OcrResponseEvent[]>([]);
  const [ocrLoading, setOcrLoading] = useState(false);
  const ocrTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Refs
  const messagesEndRef = useRef<HTMLDivElement | null>(null);
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);
  const inputContainerRef = useRef<HTMLDivElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const measurementRef = useRef<HTMLDivElement>(null);

  // Conversation Manager (replaces all conversation state/logic)
  const {
    messages,
    conversationId,
    isStreaming,
    isLoading,
    sendMessage,
    createNew,
    clear,
  } = useConversationManager(messagesEndRef);

  // Load HUD dimensions and set up settings listener
  useEffect(() => {
    const loadDimensions = async () => {
      try {
        const dimensions = await SettingsService.getHudDimensions();
        setHudDimensions(dimensions);
      } catch (error) {
        console.error('Failed to load HUD dimensions:', error);
        setHudDimensions({ width: 500, collapsed_height: 60, expanded_height: 350 });
      }
    };

    loadDimensions();

    // Set up settings change listener
    const setupListener = async () => {
      try {
        const unlisten = await listen('settings_changed', async () => {
          await loadDimensions();
          try {
            await invoke('refresh_hud_window_size', { 
              label: 'floating-hud', 
              isExpanded 
            });
          } catch (error) {
            console.error('Failed to refresh HUD window size:', error);
          }
        });
        return unlisten;
      } catch (error) {
        console.error('Failed to set up settings listener:', error);
        return null;
      }
    };

    let cleanup: UnlistenFn | null = null;
    setupListener().then((fn) => {
      cleanup = fn;
    });

    return () => {
      if (cleanup) {
        try {
          cleanup();
        } catch (error) {
          console.error('Error cleaning up settings listener:', error);
        }
      }
    };
  }, [isExpanded]);

  // Set up OCR listener
  useEffect(() => {
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
  }, []);

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

  // Window management functions
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

    setIsExpanded(true);
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

      // Dynamically resize window to fit content
      if (messagesContainerRef.current) {
        const scrollHeight = messagesContainerRef.current.scrollHeight;
        try {
          await invoke('resize_hud_dynamic', { 
            label: 'floating-hud', 
            additionalHeight: scrollHeight 
          });
        } catch (error) {
          console.error('Failed to resize window:', error);
        }
      }
    } catch (error) {
      console.error('Error in handleSubmit:', error);
    }
  }

  async function clearAndCollapse() {
    clear();
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
      await createNew();
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