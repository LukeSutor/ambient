import { ConversationList } from "@/components/hud/conversation-list";
import { MessageList } from "@/components/hud/message-list";
import type { ChatMessage } from "@/lib/conversations/types";
import { useWindows } from "@/lib/windows/useWindows";
import type { Conversation } from "@/types/conversations";
import type { HudDimensions } from "@/types/settings";
import type React from "react";
import { useCallback, useMemo } from "react";

interface DynamicChatContentProps {
  hudDimensions: HudDimensions | null;
  conversationName: string;
  messages: ChatMessage[];
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  conversations: Conversation[];
  hasMoreConversations: boolean;
  loadConversation: (id: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  loadMoreConversations: () => Promise<void>;
  renameConversation: (
    conversationId: string,
    newName: string,
  ) => Promise<void>;
  handleNewChat: () => void;
}

export function DynamicChatContent({
  hudDimensions,
  conversationName,
  messages,
  messagesEndRef,
  conversations,
  hasMoreConversations,
  loadConversation,
  deleteConversation,
  loadMoreConversations,
  renameConversation,
  handleNewChat,
}: DynamicChatContentProps) {
  // Window Manager
  const { isChatExpanded, isChatHistoryExpanded } = useWindows();

  const dynamicConversationsClass = useCallback(() => {
    if (!isChatHistoryExpanded) {
      return "w-0 max-h-0 opacity-0 pointer-events-none";
    }
    if (!isChatExpanded) {
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

  const maxHeight = useMemo(
    () => (hudDimensions ? `${hudDimensions.chat_max_height}px` : "500px"),
    [hudDimensions],
  );
  const isVisible = isChatExpanded || isChatHistoryExpanded;
  const containerClasses = useMemo(
    () =>
      `flex flex-col mx-2 transition-[max-height,opacity] duration-300 ease-in-out overflow-hidden ${isVisible ? "opacity-100" : "opacity-0 pointer-events-none"}`,
    [isVisible],
  );
  const containerStyle = useMemo<React.CSSProperties>(
    () => ({ maxHeight: isVisible ? maxHeight : "0px" }),
    [isVisible, maxHeight],
  );

  return (
    <div className={containerClasses} style={containerStyle}>
      <div
        className={`flex flex-row justify-center min-h-0 ${isChatExpanded && isChatHistoryExpanded ? "space-x-2" : ""}`}
      >
        {/* Conversation list */}
        <div
          className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicConversationsClass()}`}
        >
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
        <div
          className={`overflow-hidden transition-all duration-300 min-h-0 ${dynamicMessagesClass()}`}
        >
          <MessageList
            conversationName={conversationName}
            messages={messages}
            messagesEndRef={messagesEndRef}
            handleNewChat={handleNewChat}
          />
        </div>
      </div>
    </div>
  );
}
