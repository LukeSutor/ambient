'use client';

import React, { forwardRef } from 'react';
import Markdown from 'react-markdown';
import { llmMarkdownConfig } from '@/components/ui/markdown-config';
import AnimatedText from '@/components/ui/animated-text';
import { HoverCard, HoverCardTrigger, HoverCardContent } from '@/components/ui/hover-card';
import { Button } from '@/components/ui/button';
import { NotebookPen } from 'lucide-react';
import { ChatMessage } from '@/lib/conversations';

interface MessageListProps {
  messages: ChatMessage[];
  isChatExpanded: boolean;
  showMarkdown?: boolean; // Allow turning off markdown for perf if desired
}

// Container element forwards ref to the tail sentinel to support scrollIntoView
export const MessageList = forwardRef<HTMLDivElement, MessageListProps>(
  ({ messages, isChatExpanded, showMarkdown = true }, endRef) => {
    // Helper function to check if previous message has memory
    const hasPreviousMemory = (index: number) => {
      return index > 0 && messages[index - 1]?.role === 'user' && messages[index - 1]?.memory !== null;
    };

    return (
      <div id="message-list" className="flex flex-col space-y-2">
        {messages.map((m, i) => (
          <div
            key={`m-${i}`}
            className={
              m.role === 'user'
                ? 'max-w-[85%] ml-auto grid transition-[grid-template-rows] duration-300 ease-out'
                : 'max-w-[95%] w-full text-left mx-auto grid transition-[grid-template-rows] duration-300 ease-out'
            }
            style={{
              gridTemplateRows: m.content ? '1fr' : '0fr'
            }}
          >
            {m.role === 'user' ? (
              <div className="overflow-hidden bg-white/60 border border-black/20 rounded-xl px-3 py-2">
                <div className="whitespace-pre-wrap">{m.content}</div>
              </div>
            ) : (
              <div className="overflow-hidden">
                {/* Always reserve space for the memory indicator area */}
                <div className="h-4 flex items-center justify-start mb-1">
                  {hasPreviousMemory(i) ? (
                    <HoverCard>
                      <HoverCardTrigger asChild>
                        <div className="flex items-center gap-1 text-sm text-black/50">
                          <NotebookPen className="h-4 w-4" />
                          <span className='font-semibold'>Updated saved memory</span>
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
                              // TODO: Implement manage memories functionality
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
                {showMarkdown ? (
                  <Markdown {...llmMarkdownConfig}>{m.content}</Markdown>
                ) : (
                  <div className="prose prose-sm max-w-none">
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
