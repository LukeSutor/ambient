'use client';

import { useEffect, useRef, useState, useCallback, useMemo } from 'react';
import { HudDimensions } from '@/types/settings';
import { useSettings } from '@/lib/settings';
import HUDInputBar from '@/components/hud/hud-input-bar';
import { useConversation } from '@/lib/conversations';
import { useWindows } from '@/lib/windows/useWindows';
import { DynamicChatContent } from '@/components/hud/dynamic-chat-content';
import { AutoResizeContainer } from '@/components/hud/auto-resize-container';

export default function HudPage() {
  // UI State
  const [input, setInput] = useState('');
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(null);
  
  // Refs
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // Conversation Manager
  const {
    messages,
    conversations,
    hasMoreConversations,
    conversationId,
    ocrResults,
    ocrLoading,
    isLoading,
    isStreaming,
    sendMessage,
    resetConversation,
    loadConversation,
    loadMoreConversations,
    renameConversation,
    dispatchOCRCapture,
    deleteOCRResult,
    clearOCRResults,
    clear,
  } = useConversation(messagesEndRef);

  // Settings Manager
  const { settings, getHudDimensions } = useSettings();

  // Window Manager
  const {
    isChatExpanded,
    isChatHistoryExpanded,
    setChatMinimized,
    setChatExpanded,
    toggleChatHistory,
    closeHUD,
    openSettings,
  } = useWindows();

  // Load HUD dimensions only once on mount or when settings change
  useEffect(() => {
    let cancelled = false;
    (async () => {
      const dimensions = await getHudDimensions();
      if (!cancelled) {
        // Only update if dimensions actually changed
        setHudDimensions(prev => {
          if (!prev) return dimensions;
          // Deep comparison to avoid unnecessary updates
          if (JSON.stringify(prev) === JSON.stringify(dimensions)) {
            return prev;
          }
          return dimensions;
        });
      }
    })();
    return () => { cancelled = true; };
  }, [settings]);

  const handleMouseLeave = async (e: React.MouseEvent) => {
    setIsHoveringGroup(false);
    // Get the bounding box of drag area
    const dragArea = document.getElementById('drag-area');
    if (!dragArea) return;

    // See if mouse is within bounding box
    const rect = dragArea.getBoundingClientRect();
    let mouseCoords = { x: e.clientX, y: e.clientY };
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

  async function handleSubmit(e?: React.FormEvent) {
    if (e) {
      e.preventDefault();
    }

    const query = input.trim();

    if (!query || isLoading) return;

    await setChatExpanded();
    setInput('');

    try {
      // Send message (will create conversation if needed)
      await sendMessage(conversationId, query);
      
      // Clear OCR results after sending
      clearOCRResults();
    } catch (error) {
      console.error('Error in handleSubmit:', error);
    }
  };

  async function clearAndCollapse() {
    clear(250);
    await setChatMinimized(300);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit(e as any);
    }
    if (e.key === 'Escape') {
      handleNewChat();
    }
  };

  const handleNewChat = async () => {
    // Don't create new conversation if there are no messages
    if (messages.length > 0) {
      await clearAndCollapse();
      await resetConversation();
    }
  };

  return (
    <AutoResizeContainer hudDimensions={hudDimensions} className="bg-blue-500">
      {/* Glass Container */}
      <div className="relative flex flex-col justify-start">
        <div className="relative flex flex-col min-h-0 h-min">
          {/* Dynamic Chat Content Area */}
          <div>
            <DynamicChatContent 
              hudDimensions={hudDimensions}
              isChatExpanded={isChatExpanded}
              isChatHistoryExpanded={isChatHistoryExpanded}
              messages={messages}
              messagesEndRef={messagesEndRef}
              conversations={conversations}
              hasMoreConversations={hasMoreConversations}
              setChatExpanded={async (expanded: boolean) => { await setChatExpanded(); }}
              loadConversation={loadConversation}
              loadMoreConversations={loadMoreConversations}
              renameConversation={renameConversation}
              toggleChatHistory={toggleChatHistory}
            />
          </div>

          {/* Input Container - fixed height at bottom */}
          <HUDInputBar
            hudDimensions={hudDimensions}
            inputValue={input}
            setInputValue={setInput}
            handleSubmit={handleSubmit}
            onKeyDown={handleKeyDown}
            dispatchOCRCapture={dispatchOCRCapture}
            deleteOCRResult={deleteOCRResult}
            onNewChat={handleNewChat}
            onDragStart={() => setIsDraggingWindow(true)}
            onMouseLeave={handleMouseLeave}
            isDraggingWindow={isDraggingWindow}
            isHoveringGroup={isHoveringGroup}
            setIsHoveringGroup={setIsHoveringGroup}
            ocrLoading={ocrLoading}
            ocrResults={ocrResults}
            isStreaming={isStreaming}
            toggleChatHistory={toggleChatHistory}
            closeHUD={closeHUD}
            openSettings={openSettings}
          />
        </div>
      </div>
    </AutoResizeContainer>
  );
}