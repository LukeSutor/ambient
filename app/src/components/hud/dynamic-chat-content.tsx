import React, { useState, useEffect, useRef } from 'react';
import { HudDimensions } from '@/types/settings';
import { useWindows } from '@/lib/windows/useWindows';
import MessageList from '@/components/hud/message-list';

interface DynamicChatContentProps {
    hudDimensions: HudDimensions | null;
};

export function DynamicChatContent({ hudDimensions }: DynamicChatContentProps) {
  const {
    dynamicChatContentRef
  } = useWindows();

  return (
    <div ref={dynamicChatContentRef}>

      {/* Message list */}
      <div
      className="h-full text-black/90 text-sm leading-relaxed bg-white/60 border border-black/20 rounded-xl mx-2"
      style={{maxHeight: hudDimensions?.chat_max_height ?? 500}}
      >
        <MessageList hudDimensions={hudDimensions} />
      </div>
      {/* Dynamic chat content goes here */}
    </div>
  );
};