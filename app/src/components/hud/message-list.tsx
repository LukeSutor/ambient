'use client';

import React, { useRef } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import { HoverCard, HoverCardTrigger, HoverCardContent } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import { NotebookPen } from 'lucide-react';
import { ContentContainer } from '@/components/hud/content-container';
import { ChatMessage } from '@/lib/conversations/types';
import { useWindows } from '@/lib/windows/useWindows';

interface MessageListProps {
  messages: ChatMessage[];
  messagesEndRef: React.RefObject<HTMLDivElement | null>;
}

// Helper function to check if previous message has memory
const hasPreviousMemory = (messages: ChatMessage[], index: number) => {
  return index > 0 && messages[index - 1]?.role === 'user' && messages[index - 1]?.memory !== null;
};

function UserMessage({ message }: { message: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 max-w-[85%] ml-auto">
      <div className="whitespace-pre-wrap">{message.content}</div>
    </div>
  );
}

function AssistantMessage({ messages, message, i, openSecondary }: { messages: ChatMessage[]; message: ChatMessage ; i: number; openSecondary: (dest: string) => void }) {
  return (
    <div className="overflow-hidden">
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
      <Markdown {...llmMarkdownConfig}>{message.content}</Markdown>
    </div>
  );
}

function FunctionMessage({ message }: { message: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>{message.content}</Markdown>
    </div>
  );
}

// Container element forwards ref to the tail sentinel to support scrollIntoView
export function MessageList({ messages, messagesEndRef }: MessageListProps) {
  // Window state
  const { openSecondary } = useWindows();

  return (
    <ContentContainer>
      <div className="w-full h-full flex flex-col hud-scroll overflow-y-auto p-4">
        <div className="flex flex-col space-y-2">
          {messages.map((m, i) => (
            <div
              key={`m-${i}`}
              className={
                m.role.toLowerCase() === 'user'
                  ? 'max-w-[85%] ml-auto grid transition-[grid-template-rows] duration-300 ease-out'
                  : 'max-w-[95%] w-full text-left ml-2 mb-0 grid transition-[grid-template-rows] duration-300 ease-out'
              }
              style={{
                gridTemplateRows: m.content ? '1fr' : '0fr'
              }}
            >
              {m.role.toLowerCase() === 'user' ? (
                <UserMessage message={m} />
              ) : m.role.toLowerCase() === 'assistant' ? (
                <AssistantMessage messages={messages} message={m} i={i} openSecondary={openSecondary} />
              ) : (
                <FunctionMessage message={m} />
              )}
            </div>
          ))}
          <div ref={messagesEndRef} />
        </div>
      </div>
    </ContentContainer>
  );
}

MessageList.displayName = 'MessageList';

export default MessageList;
