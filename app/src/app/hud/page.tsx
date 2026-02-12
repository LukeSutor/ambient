"use client";

import { AutoResizeContainer } from "@/components/hud/auto-resize-container";
import { DynamicChatContent } from "@/components/hud/dynamic-chat-content";
import HUDInputBar from "@/components/hud/hud-input-bar";
import { Toaster } from "@/components/ui/sonner";

export default function HudPage() {
  return (
    <AutoResizeContainer widthType="chat">
      <Toaster richColors position="top-center" />

      <div className="flex flex-col">
        {/* Dynamic Chat Content Area */}
        <DynamicChatContent />

        {/* Input Container */}
        <HUDInputBar />
      </div>
    </AutoResizeContainer>
  );
}
