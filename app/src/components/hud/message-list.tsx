'use client';

import React, { useState } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import { HoverCard, HoverCardTrigger, HoverCardContent } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import { ChevronDown, NotebookPen } from 'lucide-react';
import { ContentContainer } from '@/components/hud/content-container';
import { ChatMessage } from '@/lib/conversations/types';
import { useWindows } from '@/lib/windows/useWindows';

interface MessageListProps {
  messages: ChatMessage[];
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
};

// Helper function to check if previous message has memory
const hasPreviousMemory = (messages: ChatMessage[], index: number) => {
  return index > 0 && messages[index - 1]?.message.role === 'user' && messages[index - 1]?.memory !== null;
};

function UserMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 ml-auto">
      <div className="whitespace-pre-wrap">{m.message.content}</div>
    </div>
  );
};

function ReasoningAssistantMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden">
      <Markdown {...llmMarkdownConfig}>{m.message.content}</Markdown>
    </div>
  );
};

function ReasoningFunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left">
      <Markdown {...llmMarkdownConfig}>{m.message.content}</Markdown>
    </div>
  );
}

function ReasoningMessages({ reasoningMessages, i, toggleReasoning, showReasoning }: { reasoningMessages: ChatMessage[], i: number, toggleReasoning: (index: number) => void, showReasoning: boolean }) {
  if (reasoningMessages.length === 0) return null;

  return (
    <div className="mt-4 -mb-4">
      <Button variant="ghost" onClick={() => toggleReasoning(i)}>
        {showReasoning ? 'Hide' : 'Show'} Thinking
        <ChevronDown className={`${showReasoning ? 'rotate-180' : ''} transition-transform`} />
      </Button>
      <div className={`grid transition-[grid-template-rows] duration-500 ${showReasoning ? 'grid-rows-[1fr]' : 'grid-rows-[0fr]'}`}>
        <div className="overflow-hidden">
          <div className="flex flex-row mt-2">
            <div className="w-[1px] bg-black/40 rounded-full ml-6 mr-4 flex-shrink-0" />
            <div className="flex-1 space-y-4">
              {reasoningMessages.map((rm, idx) => (
                <div key={`rm-${idx}`}>
              {rm.message.role.toLowerCase() === 'assistant' ? (
                <ReasoningAssistantMessage m={rm} />
              ) : (
                <ReasoningFunctionMessage m={rm} />
              )}
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

function AssistantMessage({ messages, m, i, openSecondary, toggleReasoning, showReasoning }: { messages: ChatMessage[]; m: ChatMessage ; i: number; openSecondary: (dest: string) => void; toggleReasoning: (index: number) => void; showReasoning: boolean }) {
  return (
    <div className="overflow-hidden">
      {/* Reasoning Messages */}
      <ReasoningMessages reasoningMessages={m.reasoningMessages} i={i} toggleReasoning={toggleReasoning} showReasoning={showReasoning} />
      <div className="h-4 flex items-center justify-start -mb-2">
        {hasPreviousMemory(messages, i) ? (
          <HoverCard>
            <HoverCardTrigger asChild>
              <div className="flex items-center gap-1 text-xs text-black/50">
                <NotebookPen className="h-4 w-4" />
                <span className='font-bold'>Updated saved memory</span>
              </div>
            </HoverCardTrigger>
            <HoverCardContent side="top" className="w-min max-w-80 bg-white/70">
              <div className="space-y-3">
                <div>
                  <p className="text-sm text-black">
                    {messages[i - 1]?.memory?.text || 'No memory text available'}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full bg-white/50"
                  onClick={(e) => {
                    e.preventDefault();
                    openSecondary('memories');
                  }}
                >
                  Manage Memories
                </Button>
              </div>
            </HoverCardContent>
          </HoverCard>
        ) : (
          <div className="h-4 w-4" />
        )}
      </div>
      <Markdown {...llmMarkdownConfig}>{m.message.content}</Markdown>
    </div>
  );
};

function FunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>{m.message.content}</Markdown>
    </div>
  );
};

// Container element forwards ref to the tail sentinel to support scrollIntoView
export function MessageList({ messages, messagesEndRef }: MessageListProps) {
  const [showReasoning, setShowReasoning] = useState(new Set<number>([]));
  
  // Window state
  const { openSecondary } = useWindows();

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

  return (
    <ContentContainer>
      <div className="w-full h-full flex flex-col hud-scroll overflow-y-auto p-4">
        <div className="flex flex-col space-y-2">
          {messages.map((m, i) => (
            <div
              key={`m-${i}`}
              className={
                m.message.role.toLowerCase() === 'user'
                  ? 'max-w-[85%] ml-auto grid transition-[grid-template-rows] duration-300 ease-out'
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
    </ContentContainer>
  );
};

MessageList.displayName = 'MessageList';

export default MessageList;
