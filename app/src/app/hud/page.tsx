'use client';

import { useEffect, useRef, useState } from 'react';
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
    conversationType,
    hasMoreConversations,
    conversationId,
    ocrResults,
    ocrLoading,
    isLoading,
    isStreaming,
    sendMessage,
    resetConversation,
    loadConversation,
    deleteConversation,
    loadMoreConversations,
    renameConversation,
    dispatchOCRCapture,
    deleteOCRResult,
    toggleComputerUse,
  } = useConversation(messagesEndRef);

  // Settings Manager
  const { settings, getHudDimensions } = useSettings(true);

  // Window Manager
  const {
    setChatMinimized,
    setChatExpanded,
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
      await sendMessage(conversationId, query);
    } catch (error) {
      console.error('Error in handleSubmit:', error);
    }
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
      await setChatMinimized();
      await resetConversation(500);
    }
  };

  return (
    <AutoResizeContainer hudDimensions={hudDimensions} widthType="chat" className="bg-transparent">
      {/* Glass Container */}
        <div className="flex flex-col">
          {/* Dynamic Chat Content Area */}
          <DynamicChatContent 
            hudDimensions={hudDimensions}
            messages={messages}
            messagesEndRef={messagesEndRef}
            conversations={conversations}
            hasMoreConversations={hasMoreConversations}
            loadConversation={loadConversation}
            deleteConversation={deleteConversation}
            loadMoreConversations={loadMoreConversations}
            renameConversation={renameConversation}
          />

          {/* Input Container */}
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
            toggleComputerUse={toggleComputerUse}
            ocrLoading={ocrLoading}
            ocrResults={ocrResults}
            isStreaming={isStreaming}
            conversationType={conversationType}
          />
        </div>
    </AutoResizeContainer>
  );
}