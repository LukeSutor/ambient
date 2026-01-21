"use client";

import { useState, useEffect } from 'react';
import { llmMarkdownConfig, preprocessMarkdownCurrency } from '@/components/ui/markdown-config';
import { ChatMessage } from '@/lib/conversations';
import Markdown from 'react-markdown';
import { Button } from '../ui/button';
import Image from 'next/image';
import { ChevronDown, NotebookPen, SquareDashed } from 'lucide-react';
import { HoverCard, HoverCardContent, HoverCardTrigger } from '../ui/hover-card';
import { Attachment } from '@/types/conversations';
import { appDataDir, join } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';

// Helper function to check if previous message has memory
const hasPreviousMemory = (messages: ChatMessage[], index: number) => {
  return index > 0 && messages[index - 1]?.message.role === 'user' && messages[index - 1]?.memory !== null;
};

function PreviewAttachment({ a }: { a: Attachment }) {
  const [fileSrc, setFileSrc] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(false);

  useEffect(() => {
    const resolvePath = async () => {
      if (a.file_path) {
        const appDataDirPath = await appDataDir();
        const fullPath = await join(appDataDirPath, a.file_path);
        setFileSrc(convertFileSrc(fullPath));
      }
    };
    resolvePath();
  }, [a.file_path]);
  console.log({a, fileSrc});
  if (a.file_type.startsWith('image/') && fileSrc) {
    return (
      <div className="my-2 max-w-1/2 ml-auto">
        <Image src={fileSrc} alt={a.file_name} className="h-auto rounded-md" width={400} height={400} />
      </div>
    );
  } else if (a.file_type === 'application/pdf' && fileSrc) {
    return (
      <div className={`relative flex flex-col justify-center items-center ml-auto p-4 bg-white/20 border border-black/20 rounded-lg transition-all duration-500 ${expanded ? 'w-full' : 'w-72'}`}>
        <p className="font-semibold truncate text-left w-full pr-8">{a.file_name}</p>
        <div className="flex flex-row justify-start items-center space-x-2 w-full">
          <Image src='/pdf-icon.png' alt='PDF Icon' width={16} height={16} />
          <p className="text-sm">PDF</p>
        </div>
        <div className={`grid transition-[grid-template-rows] duration-500 w-full ${expanded ? 'grid-rows-[1fr] mt-4' : 'grid-rows-[0fr]'}`}>
          <div className="overflow-hidden">
            <iframe src={fileSrc} className="w-full h-[700px] rounded-md border border-black/20" />
          </div>
        </div>
        <Button className="absolute top-2 right-2 rounded-full" variant="ghost" size="icon" onClick={() => setExpanded(!expanded)}>
          <ChevronDown className={`${expanded ? 'rotate-180' : ''} transition-transform`} />
        </Button>
      </div>
    );
  } else if (a.file_type === 'ambient/ocr' && a.extracted_text) {
    return (
      <div className={`relative flex flex-col justify-center items-center ml-auto p-4 bg-white/20 border border-black/20 rounded-lg transition-all duration-500 ${expanded ? 'w-full' : 'w-72'}`}>
        <p className="font-semibold truncate text-left w-full pr-8">{a.file_name}</p>
        <div className="flex flex-row justify-start items-center space-x-2 w-full">
          <SquareDashed className="!h-4 !w-4 text-black" />
          <p className="text-sm">Screen Capture</p>
        </div>
        <div className={`grid transition-[grid-template-rows] duration-500 w-full ${expanded ? 'grid-rows-[1fr] mt-4' : 'grid-rows-[0fr]'}`}>
          <div className="overflow-hidden">
            <p>{a.extracted_text}</p>
          </div>
        </div>
        <Button className="absolute top-2 right-2 rounded-full" variant="ghost" size="icon" onClick={() => setExpanded(!expanded)}>
          <ChevronDown className={`${expanded ? 'rotate-180' : ''} transition-transform`} />
        </Button>
      </div>
    )
  }
  return null;
}

export function UserMessage({ m }: { m: ChatMessage }) {
  return (
    <>
      {m.message.attachments?.map((a, index) => (
        <PreviewAttachment key={`attachment-${index}`} a={a} />
      ))}
      <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 ml-auto">
        <div className="whitespace-pre-wrap">{m.message.content}</div>
      </div>
    </>
  );
};

export function ReasoningAssistantMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden">
      <Markdown {...llmMarkdownConfig}>{preprocessMarkdownCurrency(m.message.content)}</Markdown>
    </div>
  );
};

export function ReasoningFunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left">
      <Markdown {...llmMarkdownConfig}>{preprocessMarkdownCurrency(m.message.content)}</Markdown>
    </div>
  );
};

export function ReasoningMessages({ reasoningMessages, i, toggleReasoning, showReasoning }: { reasoningMessages: ChatMessage[], i: number, toggleReasoning: (index: number) => void, showReasoning: boolean }) {
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

export function AssistantMessage({ messages, m, i, openSecondary, toggleReasoning, showReasoning }: { messages: ChatMessage[]; m: ChatMessage ; i: number; openSecondary: (dest: string) => void; toggleReasoning: (index: number) => void; showReasoning: boolean }) {
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
      <Markdown {...llmMarkdownConfig}>{preprocessMarkdownCurrency(m.message.content)}</Markdown>
    </div>
  );
};

export function FunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>{preprocessMarkdownCurrency(m.message.content)}</Markdown>
    </div>
  );
};