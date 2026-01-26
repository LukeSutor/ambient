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
  FunctionMessage,
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
  const [showReasoning, setShowReasoning] = useState(new Set<number>());
  const { isChatHistoryExpanded, openSecondary, toggleChatHistory } =
    useWindows();

  const toggleReasoning = useCallback((index: number) => {
    setShowReasoning((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
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
          <div className="flex flex-col space-y-2">
            {messages.map((m, i) => {
              const role = m.message.role.toLowerCase();
              const isUser = role === "user";
              const isAssistant = role === "assistant";

              return (
                <div
                  key={m.message.id}
                  className={cn(
                    "grid transition-[grid-template-rows] duration-300 ease-out",
                    isUser
                      ? "max-w-[85%] w-full ml-auto"
                      : "max-w-[95%] w-full text-left ml-2 mb-0",
                  )}
                  style={{
                    gridTemplateRows: m.message.content ? "1fr" : "0fr",
                  }}
                >
                  {isUser ? (
                    <UserMessage m={m} />
                  ) : isAssistant ? (
                    <AssistantMessage
                      messages={messages}
                      m={m}
                      i={i}
                      openSecondary={handleOpenSecondary}
                      toggleReasoning={toggleReasoning}
                      showReasoning={showReasoning.has(i)}
                    />
                  ) : (
                    <FunctionMessage m={m} />
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
