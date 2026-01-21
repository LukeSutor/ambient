'use client';

import React, { useEffect, useState } from 'react';
import Markdown from 'react-markdown';
import { UserMessage, ReasoningAssistantMessage, ReasoningFunctionMessage, ReasoningMessages, AssistantMessage, FunctionMessage } from './message-types';
import { llmMarkdownConfig, preprocessMarkdownCurrency } from '@/components/ui/markdown-config';
import { HoverCard, HoverCardTrigger, HoverCardContent } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { ChevronDown, Menu, MessageSquarePlus, NotebookPen } from 'lucide-react';
import { ContentContainer } from '@/components/hud/content-container';
import { ChatMessage } from '@/lib/conversations/types';
import { useWindows } from '@/lib/windows/useWindows';

interface MessageListProps {
  conversationName: string;
  messages: ChatMessage[];
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
  handleNewChat: () => void;
};

// Container element forwards ref to the tail sentinel to support scrollIntoView
export function MessageList({ conversationName, messages, messagesEndRef, handleNewChat }: MessageListProps) {
  const [showReasoning, setShowReasoning] = useState(new Set<number>([]));
  
  // Window state
  const { isChatHistoryExpanded, openSecondary, toggleChatHistory } = useWindows();

  const toggleReasoning = (index: number) => {
    setShowReasoning((prev) => {
      const next = new Set(prev);
      if (next.has(index)) {
        next.delete(index);
      } else {
        next.add(index);
      }
      return next;
    });
  };

  useEffect(() => {
    console.log({messages});
  }, [messages]);

  return (
    <ContentContainer>
      <div className="relative w-full h-full overflow-hidden">
        <div className="flex flex-row justify-between items-center absolute top-0 left-0 right-0 z-10 p-2 pointer-events-none">
          <Tooltip>
            <TooltipTrigger asChild>
            <Button size="icon" variant="ghost" className="pointer-events-auto" onClick={() => { toggleChatHistory(); }}>
              <Menu className="w-5 h-5" />
            </Button>
            </TooltipTrigger>
            <TooltipContent alignOffset={isChatHistoryExpanded ? -80 : -90} align="end">
              <p>{isChatHistoryExpanded ? 'Hide' : 'Expand'} Chat History</p>
            </TooltipContent>
          </Tooltip>
          {conversationName === '' ?
            <div className="h-6 w-40 rounded-lg bg-white/20 animate-pulse" />
            :
            <p className="font-bold text-center">{conversationName}</p>          
          }
          <Tooltip>
            <TooltipTrigger asChild>
            <Button size="icon" variant="ghost" className="pointer-events-auto" onClick={() => { handleNewChat(); }}>
              <MessageSquarePlus className="w-5 h-5" />
            </Button>
            </TooltipTrigger>
            <TooltipContent align="end">
              <p>New Chat</p>
            </TooltipContent>
          </Tooltip>
        </div>
        <div 
          className="w-full h-full flex flex-col hud-scroll overflow-y-auto p-4 pt-12"
          style={{
            maskImage: 'linear-gradient(to bottom, transparent 40px, black 40px)',
            WebkitMaskImage: 'linear-gradient(to bottom, transparent 40px, black 40px)'
          }}
        >
          <div className="flex flex-col space-y-2">
            {messages.map((m, i) => (
              <div
                key={`m-${i}`}
                className={
                  m.message.role.toLowerCase() === 'user'
                    ? 'max-w-[85%] w-full ml-auto grid transition-[grid-template-rows] duration-300 ease-out'
                    : 'max-w-[95%] w-full text-left ml-2 mb-0 grid transition-[grid-template-rows] duration-300 ease-out'
                }
                style={{
                  gridTemplateRows: m.message.content ? '1fr' : '0fr'
                }}
              >
                {m.message.role.toLowerCase() === 'user' ? (
                  <UserMessage m={m} />
                ) : m.message.role.toLowerCase() === 'assistant' ? (
                  <AssistantMessage 
                    messages={messages}
                    m={m}
                    i={i}
                    openSecondary={openSecondary}
                    toggleReasoning={toggleReasoning}
                    showReasoning={showReasoning.has(i)}
                  />
                ) : (
                  <FunctionMessage m={m} />
                )}
              </div>
            ))}
            <div ref={messagesEndRef} />
          </div>
        </div>
      </div>
    </ContentContainer>
  );
};

MessageList.displayName = 'MessageList';

export default MessageList;
