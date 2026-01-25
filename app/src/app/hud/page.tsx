"use client";

import { AutoResizeContainer } from "@/components/hud/auto-resize-container";
import { DynamicChatContent } from "@/components/hud/dynamic-chat-content";
import HUDInputBar from "@/components/hud/hud-input-bar";
import { Toaster } from "@/components/ui/sonner";
import { useConversation } from "@/lib/conversations";
import { useSettings } from "@/lib/settings";
import { useWindows } from "@/lib/windows/useWindows";
import type { HudDimensions } from "@/types/settings";
import { useEffect, useRef, useState } from "react";

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

  // Load HUD dimensions only once on mount or when settings change
  useEffect(() => {
    void (async () => {
      const dimensions = await getHudDimensions();
      // Only update if dimensions actually changed
      setHudDimensions((prev) => {
        if (prev === null) return dimensions;
        // Deep comparison to avoid unnecessary updates
        if (JSON.stringify(prev) === JSON.stringify(dimensions)) {
          return prev;
        }
        return dimensions;
      });
    })();
  }, [getHudDimensions]);

  const handleMouseLeave = async (e: React.MouseEvent) => {
    setIsHoveringGroup(false);
    // Get the bounding box of drag area
    const dragArea = document.getElementById("drag-area");
    if (!dragArea) return;

    // See if mouse is within bounding box
    const rect = dragArea.getBoundingClientRect();
    const mouseCoords = { x: e.clientX, y: e.clientY };
    const isWithinBox =
      mouseCoords.x >= rect.left &&
      mouseCoords.x <= rect.right &&
      mouseCoords.y >= rect.top &&
      mouseCoords.y <= rect.bottom;

    // set dragging off if not within bounding box
    if (!isWithinBox) {
      setIsDraggingWindow(false);
    }
  };

  // Ensure drag visibility resets when pointer is released anywhere
  useEffect(() => {
    const onUp = () => {
      setIsDraggingWindow(false);
    };
    window.addEventListener("pointerup", onUp);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("pointerup", onUp);
      window.removeEventListener("mouseup", onUp);
    };
  }, []);

  async function handleSubmit(e?: React.FormEvent | React.KeyboardEvent) {
    if (e) {
      e.preventDefault();
    }

    const query = input.trim();

    if (!query || isLoading) return;

    setChatExpanded();
    setInput("");

    try {
      await sendMessage(conversationId, query);
    } catch (error) {
      console.error("Error in handleSubmit:", error);
    }
  }

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void handleSubmit(e);
    }
    if (e.key === "Escape") {
      void handleNewChat();
    }
  };

  const handleNewChat = async () => {
    // Don't create new conversation if there are no messages
    if (messages.length > 0) {
      setChatMinimized();
      await resetConversation(500);
    }
  };

  return (
    <AutoResizeContainer
      widthType="chat"
    >
      <Toaster richColors position="top-center" />

      {/* Glass Container */}
      <div className="flex flex-col">
        {/* Dynamic Chat Content Area */}
        <DynamicChatContent
          hudDimensions={hudDimensions}
          conversationName={conversationName}
          messages={messages}
          messagesEndRef={messagesEndRef}
          conversations={conversations}
          hasMoreConversations={hasMoreConversations}
          loadConversation={async (id) => {
            await loadConversation(id);
          }}
          deleteConversation={async (id) => {
            await deleteConversation(id);
          }}
          loadMoreConversations={async () => {
            await loadMoreConversations();
          }}
          renameConversation={async (id, name) => {
            await renameConversation(id, name);
          }}
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
          dispatchOCRCapture={() => {
            void dispatchOCRCapture();
          }}
          onDragStart={() => {
            setIsDraggingWindow(true);
          }}
          onMouseLeave={(e) => {
            void handleMouseLeave(e);
          }}
          isDraggingWindow={isDraggingWindow}
          isHoveringGroup={isHoveringGroup}
          setIsHoveringGroup={setIsHoveringGroup}
          toggleComputerUse={() => {
            toggleComputerUse();
          }}
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
