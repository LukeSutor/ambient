import React, { useCallback } from 'react';
import { HudDimensions } from '@/types/settings';
import { MessageList } from '@/components/hud/message-list';
import { ConversationList } from '@/components/hud/conversation-list';
import { ChatMessage } from '@/lib/conversations/types';
import { Conversation } from '@/types/conversations';
import { useWindows } from '@/lib/windows/useWindows';

interface DynamicChatContentProps {
    hudDimensions: HudDimensions | null;
    messages: ChatMessage[];
    messagesEndRef: React.RefObject<HTMLDivElement | null>;
    conversations: Conversation[];
    hasMoreConversations: boolean;
    loadConversation: (id: string) => Promise<void>;
    loadMoreConversations: () => Promise<void>;
    renameConversation: (conversationId: string, newName: string) => Promise<void>;
};

export function DynamicChatContent({ 
  hudDimensions, 
  messages,
  messagesEndRef,
  conversations,
  hasMoreConversations,
  loadConversation,
  loadMoreConversations,
  renameConversation,
}: DynamicChatContentProps) {
  // Window Manager
  const {
    isChatExpanded,
    isChatHistoryExpanded,
  } = useWindows();

  const dynamicConversationsClass = useCallback(() => {
    if (!isChatHistoryExpanded) {
      return "w-0 max-h-0 opacity-0";
    } else if (!isChatExpanded) {
      return "w-full max-h-full opacity-100";
    }
    return "w-[60%] max-h-full opacity-100"
  }, [isChatExpanded, isChatHistoryExpanded])

  const dynamicMessagesClass = useCallback(() => {
    if (!isChatExpanded) {
      return "w-0 max-h-0 opacity-0";
    } else if (!isChatHistoryExpanded) {
      return "w-full max-h-full opacity-100";
    }
    return "w-full max-h-full opacity-100";
  }, [isChatExpanded, isChatHistoryExpanded])

  const maxHeight = hudDimensions ? `${hudDimensions.chat_max_height}px` : '500px';

  return (
    <div className={`flex flex-col mx-2 ${(isChatExpanded || isChatHistoryExpanded) ? "" : "w-0 h-0"}`}
      style={{maxHeight}}
      >
      <div className={`flex flex-row justify-center min-h-0 ${isChatExpanded && isChatHistoryExpanded ? "space-x-2" : ""}`}>
        {/* Conversation list */}
        <div className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicConversationsClass()}`}>
          <ConversationList 
            conversations={conversations}
            hasMoreConversations={hasMoreConversations}
            loadConversation={loadConversation}
            loadMoreConversations={loadMoreConversations}
            renameConversation={renameConversation}
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