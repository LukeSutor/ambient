import React, { useState, useEffect, useRef, useCallback } from 'react';
import { HudDimensions } from '@/types/settings';
import { MessageList } from '@/components/hud/message-list';
import { ConversationList } from '@/components/hud/conversation-list';
import { ChatMessage } from '@/lib/conversations/types';
import { Conversation } from '@/types/conversations';

interface DynamicChatContentProps {
    hudDimensions: HudDimensions | null;
    isChatExpanded: boolean;
    isChatHistoryExpanded: boolean;
    messages: ChatMessage[];
    messagesEndRef: React.RefObject<HTMLDivElement | null>;
    getConversations: (limit: number, offset: number) => Promise<Conversation[]>;
    toggleChatHistory: (nextState?: boolean) => Promise<void>;
};

export function DynamicChatContent({ 
  hudDimensions, 
  isChatExpanded, 
  isChatHistoryExpanded,
  messages,
  messagesEndRef,
  getConversations,
  toggleChatHistory,
}: DynamicChatContentProps) {

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
    <div className={`flex flex-col mx-2 ${(isChatExpanded || isChatHistoryExpanded) ? "" : "w-0 h-0"}`}
      style={{maxHeight}}
      >
      <div className={`flex flex-row min-h-0 ${isChatExpanded && isChatHistoryExpanded ? "space-x-2" : ""}`}>
        {/* Conversation list */}
        <div className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicConversationsClass()}`}>
          <ConversationList 
            getConversations={getConversations}
            toggleChatHistory={toggleChatHistory}
          />
        </div>

        {/* Message list */}
        <div className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicMessagesClass()}`}>
          <MessageList 
            messages={messages}
            messagesEndRef={messagesEndRef}
          />
        </div>
      </div>
    </div>
  );
};