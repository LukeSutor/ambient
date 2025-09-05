'use client';

import React, { forwardRef } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import AnimatedText from '@/components/ui/animated-text';

export type ChatMessage = { role: 'user' | 'assistant'; content: string };

interface MessageListProps {
  messages: ChatMessage[];
  showMarkdown?: boolean; // Allow turning off markdown for perf if desired
}

// Container element forwards ref to the tail sentinel to support scrollIntoView
export const MessageList = forwardRef<HTMLDivElement, MessageListProps>(
  ({ messages, showMarkdown = true }, endRef) => {
    return (
      <div className="flex flex-col space-y-2">
        {messages.map((m, i) => (
          <div
            key={`m-${i}`}
            className={
              m.role === 'user'
                ? 'max-w-[85%] ml-auto bg-white/60 border border-black/20 rounded-xl px-3 py-2'
                : 'max-w-[95%] w-full text-left mx-auto'
            }
          >
            {m.role === 'user' ? (
              <div className="whitespace-pre-wrap">{m.content}</div>
            ) : showMarkdown ? (
              <Markdown {...llmMarkdownConfig}>{m.content}</Markdown>
            ) : (
              <div className="prose prose-sm max-w-none">
                <AnimatedText content={m.content} />
              </div>
            )}
          </div>
        ))}
        <div ref={endRef} />
      </div>
    );
  }
);

MessageList.displayName = 'MessageList';

export default MessageList;
