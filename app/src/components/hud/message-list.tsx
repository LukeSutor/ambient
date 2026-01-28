"use client";

import { ContentContainer } from "@/components/hud/content-container";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import type { ChatMessage } from "@/lib/conversations/types";
import { cn } from "@/lib/utils";
import { useWindows } from "@/lib/windows/useWindows";
import { Menu, MessageSquarePlus } from "lucide-react";
import type React from "react";
import { useCallback, useState } from "react";
import {
  AssistantMessage,
  ThinkingBlock,
  UserMessage,
} from "./message-types";

interface MessageListProps {
  conversationName: string;
  messages: ChatMessage[];
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  handleNewChat: () => void;
}

const SCROLL_MASK_STYLE = {
  maskImage: "linear-gradient(to bottom, transparent 40px, black 50px)",
  WebkitMaskImage: "linear-gradient(to bottom, transparent 40px, black 50px)",
} as const;

export function MessageList({
  conversationName,
  messages,
  messagesEndRef,
  handleNewChat,
}: MessageListProps) {
  const [showReasoning, setShowReasoning] = useState(new Set<string>());
  const { isChatHistoryExpanded, openSecondary, toggleChatHistory } =
    useWindows();

  const toggleReasoning = useCallback((id: string) => {
    setShowReasoning((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const handleToggleChatHistory = useCallback(() => {
    void toggleChatHistory();
  }, [toggleChatHistory]);

  const handleOpenSecondary = useCallback(
    (dest: string) => {
      void openSecondary(dest);
    },
    [openSecondary],
  );

  const isLoading = conversationName === "";

  // Grouping logic for thinking messages
  const groupedMessages: Array<
    | { type: "message"; message: ChatMessage }
    | { type: "thinking"; messages: ChatMessage[] }
  > = [];
  let currentThinkingGroup: ChatMessage[] = [];

  for (const m of messages) {
    const mType = (m.message.message_type || "").toLowerCase();
    const isThinking =
      mType === "thinking" ||
      mType === "tool_call" ||
      mType === "toolcall" ||
      mType === "tool_result" ||
      mType === "toolresult" ||
      (m.message.role.toLowerCase() === "assistant" && !m.message.content) ||
      m.message.role.toLowerCase() === "tool";

    if (isThinking) {
      currentThinkingGroup.push(m);
    } else {
      if (currentThinkingGroup.length > 0) {
        groupedMessages.push({
          type: "thinking",
          messages: [...currentThinkingGroup],
        });
        currentThinkingGroup = [];
      }
      groupedMessages.push({ type: "message", message: m });
    }
  }

  if (currentThinkingGroup.length > 0) {
    groupedMessages.push({
      type: "thinking",
      messages: currentThinkingGroup,
    });
  }

  return (
    <ContentContainer>
      <div className="relative w-full h-full overflow-hidden">
        {/* Header */}
        <div className="flex flex-row justify-between items-center absolute top-0 left-0 right-0 z-10 p-2 pointer-events-none">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="icon"
                variant="ghost"
                className="pointer-events-auto"
                onClick={handleToggleChatHistory}
              >
                <Menu className="w-5 h-5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent
              alignOffset={isChatHistoryExpanded ? -80 : -90}
              align="end"
            >
              <p>{isChatHistoryExpanded ? "Hide" : "Expand"} Chat History</p>
            </TooltipContent>
          </Tooltip>

          {isLoading ? (
            <div className="h-6 w-40 rounded-lg bg-white/20 animate-pulse" />
          ) : (
            <p className="font-bold text-center">{conversationName}</p>
          )}

          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                size="icon"
                variant="ghost"
                className="pointer-events-auto"
                onClick={handleNewChat}
              >
                <MessageSquarePlus className="w-5 h-5" />
              </Button>
            </TooltipTrigger>
            <TooltipContent align="end">
              <p>New Chat</p>
            </TooltipContent>
          </Tooltip>
        </div>

        {/* Message List */}
        <div
          className="w-full h-full flex flex-col hud-scroll overflow-y-auto p-4 pt-12"
          style={SCROLL_MASK_STYLE}
        >
          <div className="flex flex-col">
            {groupedMessages.map((group, groupIdx) => {
              if (group.type === "thinking") {
                const firstId = group.messages[0].message.id;
                return (
                  <div
                    key={`thinking-${firstId}`}
                    className="max-w-[95%] w-full text-left ml-2"
                  >
                    <ThinkingBlock
                      messages={group.messages}
                      isExpanded={showReasoning.has(firstId)}
                      onToggle={() => toggleReasoning(firstId)}
                    />
                  </div>
                );
              }

              const m = group.message;
              const role = m.message.role.toLowerCase();
              const isUser = role === "user";

              return (
                <div
                  key={m.message.id}
                  className={cn(
                    "grid transition-[grid-template-rows] duration-300 ease-out",
                    isUser
                      ? "max-w-full w-full"
                      : "max-w-[95%] w-full text-left ml-2 mb-0",
                  )}
                  style={{
                    gridTemplateRows: m.message.content ? "1fr" : "0fr",
                  }}
                >
                  {isUser ? (
                    <UserMessage m={m} openSecondary={handleOpenSecondary} />
                  ) : (
                    <AssistantMessage m={m} />
                  )}
                </div>
              );
            })}
            <div ref={messagesEndRef} />
          </div>
        </div>
      </div>
    </ContentContainer>
  );
}

MessageList.displayName = "MessageList";

export default MessageList;
