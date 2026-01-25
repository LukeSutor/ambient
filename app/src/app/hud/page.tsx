"use client";

import { AutoResizeContainer } from "@/components/hud/auto-resize-container";
import { DynamicChatContent } from "@/components/hud/dynamic-chat-content";
import HUDInputBar from "@/components/hud/hud-input-bar";
import { Toaster } from "@/components/ui/sonner";
import { useConversation } from "@/lib/conversations";
import { useSettings } from "@/lib/settings";
import { useWindows } from "@/lib/windows/useWindows";
import type { HudDimensions } from "@/types/settings";
import { useCallback, useEffect, useRef, useState } from "react";

export default function HudPage() {
  // UI State
  const [input, setInput] = useState("");
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(
    null,
  );

  // Refs
  const messagesEndRef = useRef<HTMLDivElement | null>(null);

  // Conversation Manager
  const {
    messages,
    conversationName,
    conversations,
    conversationType,
    hasMoreConversations,
    conversationId,
    attachmentData,
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
    toggleComputerUse,
    addAttachmentData,
    removeAttachmentData,
  } = useConversation(messagesEndRef);

  // Settings Manager
  const { getHudDimensions } = useSettings(true);

  // Window Manager
  const { setChatMinimized, setChatExpanded } = useWindows();

  // Load HUD dimensions on mount or when settings change
  useEffect(() => {
    const loadDimensions = async () => {
      const dimensions = await getHudDimensions();
      setHudDimensions((prev) => {
        if (prev === null) return dimensions;
        if (JSON.stringify(prev) === JSON.stringify(dimensions)) return prev;
        return dimensions;
      });
    };
    void loadDimensions();
  }, [getHudDimensions]);

  // Reset drag state on pointer/mouse up
  useEffect(() => {
    const handlePointerUp = () => {
      setIsDraggingWindow(false);
    };
    window.addEventListener("pointerup", handlePointerUp);
    window.addEventListener("mouseup", handlePointerUp);
    return () => {
      window.removeEventListener("pointerup", handlePointerUp);
      window.removeEventListener("mouseup", handlePointerUp);
    };
  }, []);

  const handleMouseLeave = useCallback((e: React.MouseEvent) => {
    setIsHoveringGroup(false);
    const dragArea = document.getElementById("drag-area");
    if (!dragArea) return;

    const rect = dragArea.getBoundingClientRect();
    const isWithinDragArea =
      e.clientX >= rect.left &&
      e.clientX <= rect.right &&
      e.clientY >= rect.top &&
      e.clientY <= rect.bottom;

    if (!isWithinDragArea) {
      setIsDraggingWindow(false);
    }
  }, []);

  const handleSubmit = useCallback(async () => {
    const query = input.trim();
    if (!query || isLoading) return;

    setChatExpanded();
    setInput("");

    try {
      await sendMessage(conversationId, query);
    } catch (error) {
      console.error("Error in handleSubmit:", error);
    }
  }, [input, isLoading, conversationId, sendMessage, setChatExpanded]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit],
  );

  const handleNewChat = useCallback(async () => {
    if (messages.length > 0) {
      setChatMinimized();
      await resetConversation(500);
    }
  }, [messages.length, setChatMinimized, resetConversation]);

  const handleDragStart = useCallback(() => {
    setIsDraggingWindow(true);
  }, []);

  const handleDispatchOCRCapture = useCallback(() => {
    void dispatchOCRCapture();
  }, [dispatchOCRCapture]);

  return (
    <AutoResizeContainer widthType="chat">
      <Toaster richColors position="top-center" />

      <div className="flex flex-col">
        {/* Dynamic Chat Content Area */}
        <DynamicChatContent
          hudDimensions={hudDimensions}
          conversationName={conversationName}
          messages={messages}
          messagesEndRef={messagesEndRef}
          conversations={conversations}
          hasMoreConversations={hasMoreConversations}
          loadConversation={loadConversation}
          deleteConversation={deleteConversation}
          loadMoreConversations={loadMoreConversations}
          renameConversation={renameConversation}
          handleNewChat={() => {
            void handleNewChat();
          }}
        />

        {/* Input Container */}
        <HUDInputBar
          hudDimensions={hudDimensions}
          inputValue={input}
          setInputValue={setInput}
          handleSubmit={handleSubmit}
          onKeyDown={handleKeyDown}
          dispatchOCRCapture={handleDispatchOCRCapture}
          onDragStart={handleDragStart}
          onMouseLeave={handleMouseLeave}
          isDraggingWindow={isDraggingWindow}
          isHoveringGroup={isHoveringGroup}
          setIsHoveringGroup={setIsHoveringGroup}
          toggleComputerUse={toggleComputerUse}
          ocrLoading={ocrLoading}
          isStreaming={isStreaming}
          conversationType={conversationType}
          attachmentData={attachmentData}
          addAttachmentData={addAttachmentData}
          removeAttachmentData={removeAttachmentData}
        />
      </div>
    </AutoResizeContainer>
  );
}
