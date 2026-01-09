import React, { useCallback, useMemo } from 'react';
import { HudDimensions } from '@/types/settings';
import { MessageList } from '@/components/hud/message-list';
import { ConversationList } from '@/components/hud/conversation-list';
import { ChatMessage } from '@/lib/conversations/types';
import { Conversation } from '@/types/conversations';
import { useWindows } from '@/lib/windows/useWindows';

interface DynamicChatContentProps {
    hudDimensions: HudDimensions | null;
    messages: ChatMessage[];
    reasoningMessages: ChatMessage[];
    messagesEndRef: React.RefObject<HTMLDivElement | null>;
    conversations: Conversation[];
    hasMoreConversations: boolean;
    loadConversation: (id: string) => Promise<void>;
    deleteConversation: (id: string) => Promise<void>;
    loadMoreConversations: () => Promise<void>;
    renameConversation: (conversationId: string, newName: string) => Promise<void>;
};

export function DynamicChatContent({ 
  hudDimensions, 
  messages,
  reasoningMessages,
  messagesEndRef,
  conversations,
  hasMoreConversations,
  loadConversation,
  deleteConversation,
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
      return "w-0 max-h-0 opacity-0 pointer-events-none";
    } else if (!isChatExpanded) {
      return "w-full max-h-96 opacity-100";
    }
    return "w-[60%] min-h-32 max-h-full opacity-100";
  }, [isChatExpanded, isChatHistoryExpanded]);

  const dynamicMessagesClass = useCallback(() => {
    if (!isChatExpanded) {
      return "w-0 max-h-0 opacity-0 pointer-events-none";
    }
    return "w-full max-h-full opacity-100";
  }, [isChatExpanded]);

  const maxHeight = useMemo(() => hudDimensions ? `${hudDimensions.chat_max_height}px` : '500px', [hudDimensions]);
  const isVisible = isChatExpanded || isChatHistoryExpanded;
  const containerClasses = useMemo(() => `flex flex-col mx-2 transition-[max-height,opacity] duration-300 ease-in-out overflow-hidden ${isVisible ? 'opacity-100' : 'opacity-0 pointer-events-none'}`, [isVisible]);
  const containerStyle = useMemo<React.CSSProperties>(() => ({ maxHeight: isVisible ? maxHeight : '0px' }), [isVisible, maxHeight]);

  return (
    <div className={containerClasses}
      style={containerStyle}
      >
      <div className={`flex flex-row justify-center min-h-0 ${isChatExpanded && isChatHistoryExpanded ? "space-x-2" : ""}`}>
        {/* Conversation list */}
        <div className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicConversationsClass()}`}>
          <ConversationList 
            conversations={conversations}
            hasMoreConversations={hasMoreConversations}
            loadConversation={loadConversation}
            deleteConversation={deleteConversation}
            loadMoreConversations={loadMoreConversations}
            renameConversation={renameConversation}
          />
        </div>

        {/* Message list */}
        <div className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicMessagesClass()}`}>
          <MessageList 
            messages={messages}
            reasoningMessages={reasoningMessages}
            messagesEndRef={messagesEndRef}
          />
        </div>
      </div>
    </div>
  );
};