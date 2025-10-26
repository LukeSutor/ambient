import React, { useState, useEffect, useRef, useCallback } from 'react';
import { HudDimensions } from '@/types/settings';
import { useWindows } from '@/lib/windows/useWindows';
import { MessageList } from '@/components/hud/message-list';
import { ConversationList } from '@/components/hud/conversation-list';

interface DynamicChatContentProps {
    hudDimensions: HudDimensions | null;
};

export function DynamicChatContent({ hudDimensions }: DynamicChatContentProps) {
  const {
    isChatExpanded,
    isChatHistoryExpanded
  } = useWindows();

  const dynamicConversationsClass = useCallback(() => {
    if (!isChatHistoryExpanded) {
      return "w-0 h-0";
    } else if (!isChatExpanded) {
      return "w-full"
    }
    return "w-[60%]"
  }, [isChatExpanded, isChatHistoryExpanded])

  const dynamicMessagesClass = useCallback(() => {
    if (!isChatExpanded) {
      return "w-0 h-0";
    } else if (!isChatHistoryExpanded) {
      return "w-full"
    }
    return "w-full"
  }, [isChatExpanded, isChatHistoryExpanded])

  const maxHeight = hudDimensions ? `${hudDimensions.chat_max_height}px` : '500px';

  return (
    <div className={`flex flex-col overflow-hidden mx-2 ${(isChatExpanded || isChatHistoryExpanded) ? "" : "w-0 h-0"}`}
      style={{maxHeight}}
      >
      <div className={`flex flex-row h-full ${isChatExpanded && isChatHistoryExpanded ? "space-x-2" : ""}`}>
        {/* Conversation list */}
        <div className={`overflow-hidden transition-all duration-300 ${dynamicConversationsClass()}`}>
          <ConversationList hudDimensions={hudDimensions} />
        </div>

        {/* Message list */}
        <div className={`overflow-hidden transition-all duration-300 ${dynamicMessagesClass()}`}>
          <MessageList hudDimensions={hudDimensions} />
        </div>
      </div>
    </div>
  );
};