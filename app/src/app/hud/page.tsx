"use client";

import { AutoResizeContainer } from "@/components/hud/auto-resize-container";
import { DynamicChatContent } from "@/components/hud/dynamic-chat-content";
import HUDInputBar from "@/components/hud/hud-input-bar";
import { Toaster } from "@/components/ui/sonner";
import { useCallback, useEffect, useState } from "react";

export default function HudPage() {
  // UI State
  const [isDraggingWindow, setIsDraggingWindow] = useState(false);
  const [isHoveringGroup, setIsHoveringGroup] = useState(false);

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

  const handleDragStart = useCallback(() => {
    setIsDraggingWindow(true);
  }, []);

  return (
    <AutoResizeContainer widthType="chat">
      <Toaster richColors position="top-center" />

      <div className="flex flex-col">
        {/* Dynamic Chat Content Area */}
        <DynamicChatContent />

        {/* Input Container */}
        <HUDInputBar
          onDragStart={handleDragStart}
          onMouseLeave={handleMouseLeave}
          isDraggingWindow={isDraggingWindow}
          isHoveringGroup={isHoveringGroup}
          setIsHoveringGroup={setIsHoveringGroup}
        />
      </div>
    </AutoResizeContainer>
  );
}
