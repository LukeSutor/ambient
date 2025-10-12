'use client';

import React, { forwardRef } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import AnimatedText from '@/components/ui/animated-text';
import { MemoryEntry } from '@/types/memory';
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip';
import { NotebookPen } from 'lucide-react';

export type ChatMessage = { role: 'user' | 'assistant'; content: string; memory: MemoryEntry | null };

interface MessageListProps {
  messages: ChatMessage[];
  showMarkdown?: boolean; // Allow turning off markdown for perf if desired
}

// Container element forwards ref to the tail sentinel to support scrollIntoView
export const MessageList = forwardRef<HTMLDivElement, MessageListProps>(
  ({ messages, showMarkdown = true }, endRef) => {
    console.log('Rendering MessageList with messages:', messages);
    
    // Helper function to check if previous message has memory
    const hasPreviousMemory = (index: number) => {
      return index > 0 && messages[index - 1]?.role === 'user' && messages[index - 1]?.memory !== null;
    };

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
            ) : (
              <div>
                {/* Always reserve space for the tooltip area */}
                <div className="h-6 flex items-center justify-start mb-1">
                  {hasPreviousMemory(i) ? (
                    <Tooltip>
                      <TooltipTrigger asChild>
                        <div className="flex items-center gap-1 text-muted-foreground cursor-help">
                          <NotebookPen className="h-4 w-4" />
                        </div>
                      </TooltipTrigger>
                      <TooltipContent side="top">
                        <p>Updated saved memory</p>
                      </TooltipContent>
                    </Tooltip>
                  ) : (
                    <div className="h-4 w-4" />
                  )}
                </div>
                {showMarkdown ? (
                  <Markdown {...llmMarkdownConfig}>{m.content}</Markdown>
                ) : (
                  <div className="prose prose-sm max-w-none">
                    {messages[i-1]?.memory && (
                      <div className="mb-2 p-2 bg-yellow-50 border-l-4 border-yellow-400">
                        <p className="font-semibold">Context from memory:</p>
                        <p className="whitespace-pre-wrap">{messages[i-1].memory?.text}</p>
                      </div>
                    )}
                    <p>assistant</p>
                    <AnimatedText content={m.content} />
                  </div>
                )}
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
