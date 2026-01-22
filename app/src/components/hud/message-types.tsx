"use client";

import {
  llmMarkdownConfig,
  preprocessMarkdownCurrency,
} from "@/components/ui/markdown-config";
import type { ChatMessage } from "@/lib/conversations";
import type { Attachment } from "@/types/conversations";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import {
  Camera,
  ChevronDown,
  FileText,
  NotebookPen,
  Search,
  SquareDashed,
} from "lucide-react";
import Image from "next/image";
import { useEffect, useState } from "react";
import Markdown from "react-markdown";
import { Button } from "../ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "../ui/dialog";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "../ui/hover-card";

// Helper function to check if previous message has memory
const hasPreviousMemory = (messages: ChatMessage[], index: number) => {
  return (
    index > 0 &&
    messages[index - 1]?.message.role === "user" &&
    messages[index - 1]?.memory !== null
  );
};

function PreviewAttachment({ a }: { a: Attachment }) {
  const [fileSrc, setFileSrc] = useState<string | null>(null);

  useEffect(() => {
    const resolvePath = async () => {
      if (a.file_path) {
        const appDataDirPath = await appDataDir();
        const fullPath = await join(appDataDirPath, a.file_path);
        setFileSrc(convertFileSrc(fullPath));
      }
    };
    void resolvePath();
  }, [a.file_path]);

  if (a.file_type.startsWith("image/") && fileSrc) {
    return (
      <div className="my-2 max-w-[80%] ml-auto">
        <Dialog>
          <DialogTrigger asChild>
            <button
              type="button"
              className="relative w-full group outline-none"
            >
              <Image
                src={fileSrc}
                alt={a.file_name}
                className="h-auto rounded-lg transition-all group-hover:brightness-75"
                width={400}
                height={400}
                unoptimized
              />
              <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
                <Search className="w-8 h-8 text-white drop-shadow-md" />
              </div>
            </button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-[90vw] h-[90vh] p-0 overflow-hidden border-none shadow-2xl bg-zinc-100 flex flex-col gap-0">
            <DialogDescription className="sr-only">
              Preview of {a.file_name}
            </DialogDescription>
            <DialogHeader className="shrink-0 p-4 border-b bg-white flex flex-row items-center justify-between space-y-0">
              <DialogTitle className="text-sm truncate font-bold flex items-center gap-2 pr-8">
                <Camera className="h-4 w-4 text-emerald-800" />
                {a.file_name}
              </DialogTitle>
            </DialogHeader>
            <div className="flex-1 w-full p-4 flex items-center justify-center bg-zinc-100/50 min-h-0">
              <img
                src={fileSrc}
                alt={a.file_name}
                className="max-w-full max-h-full object-contain rounded-lg"
              />
            </div>
          </DialogContent>
        </Dialog>
      </div>
    );
  }

  if (a.file_type === "application/pdf" && fileSrc) {
    return (
      <div className="ml-auto w-full max-w-[280px] my-2">
        <Dialog>
          <DialogTrigger asChild>
            <button
              type="button"
              className="flex items-center gap-3 p-3 bg-white/40 border border-black/10 rounded-lg hover:bg-white/60 transition-all active:scale-[0.98] w-full text-left group"
            >
              <div className="h-10 w-10 flex items-center justify-center bg-red-500/10 rounded-lg flex-shrink-0 group-hover:bg-red-500/20 transition-colors">
                <Image
                  src="/pdf-icon.png"
                  alt="PDF Icon"
                  width={20}
                  height={20}
                />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold truncate text-black/80">
                  {a.file_name}
                </p>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <span className="text-[10px] bg-red-500/10 text-red-600 px-1.5 py-0.5 rounded font-bold uppercase">
                    PDF
                  </span>
                  <span className="text-[11px] text-black/40">
                    Click to preview
                  </span>
                </div>
              </div>
              <div className="h-8 w-8 flex items-center justify-center rounded-full bg-black/5 opacity-0 group-hover:opacity-100 transition-opacity">
                <FileText className="w-4 h-4 text-black/40" />
              </div>
            </button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-[90vw] h-[90vh] p-0 overflow-hidden border-none shadow-2xl bg-zinc-100 flex flex-col gap-0">
            <DialogDescription className="sr-only">
              Preview of {a.file_name}
            </DialogDescription>
            <DialogHeader className="shrink-0 p-4 border-b bg-white flex flex-row items-center justify-between space-y-0">
              <DialogTitle className="text-sm truncate font-bold flex items-center gap-2 pr-8">
                <Image src="/pdf-icon.png" alt="PDF" width={16} height={16} />
                {a.file_name}
              </DialogTitle>
            </DialogHeader>
            <div className="flex-1 w-full p-4 min-h-0">
              <iframe
                title={`PDF Preview of ${a.file_name}`}
                src={fileSrc}
                className="w-full h-full border rounded-lg bg-white shadow-inner"
              />
            </div>
          </DialogContent>
        </Dialog>
      </div>
    );
  }

  if (a.file_type === "ambient/ocr" && a.extracted_text) {
    return (
      <div className="ml-auto w-full max-w-[280px] my-2">
        <Dialog>
          <DialogTrigger asChild>
            <button
              type="button"
              className="flex items-center gap-3 p-3 bg-white/40 border border-black/10 rounded-lg hover:bg-white/60 transition-all active:scale-[0.98] w-full text-left group"
            >
              <div className="h-10 w-10 flex items-center justify-center bg-blue-500/10 rounded-lg flex-shrink-0 group-hover:bg-blue-500/20 transition-colors">
                <SquareDashed className="h-5 w-5 text-blue-600" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-semibold truncate text-black/80">
                  {a.file_name || "Screen Capture"}
                </p>
                <div className="flex items-center gap-1.5 mt-0.5">
                  <span className="text-[10px] bg-blue-500/10 text-blue-600 px-1.5 py-0.5 rounded font-bold uppercase">
                    OCR
                  </span>
                  <span className="text-[11px] text-black/40">
                    Click to view text
                  </span>
                </div>
              </div>
              <div className="h-8 w-8 flex items-center justify-center rounded-full bg-black/5 opacity-0 group-hover:opacity-100 transition-opacity">
                <FileText className="w-4 h-4 text-black/40" />
              </div>
            </button>
          </DialogTrigger>
          <DialogContent className="sm:max-w-[90vw] h-[90vh] p-0 overflow-hidden border-none shadow-2xl bg-zinc-100 flex flex-col gap-0">
            <DialogDescription className="sr-only">
              Preview of {a.file_name}
            </DialogDescription>
            <DialogHeader className="shrink-0 p-4 border-b bg-white flex flex-row items-center justify-between space-y-0">
              <DialogTitle className="text-sm truncate font-bold flex items-center gap-2 pr-8">
                <SquareDashed className="h-4 w-4 text-blue-600" />
                {a.file_name || "Screen Capture"}
              </DialogTitle>
            </DialogHeader>
            <div className="flex-1 w-full p-8 flex items-center justify-center bg-zinc-100/50 min-h-0">
              <div className="w-full max-w-2xl bg-white p-8 rounded-lg border shadow-sm h-full overflow-y-auto">
                <pre className="text-sm leading-relaxed text-black/70 font-mono whitespace-pre-wrap">
                  {a.extracted_text}
                </pre>
              </div>
            </div>
          </DialogContent>
        </Dialog>
      </div>
    );
  }
  return null;
}

export function UserMessage({ m }: { m: ChatMessage }) {
  return (
    <>
      {m.message.attachments.map((a) => (
        <PreviewAttachment key={a.id} a={a} />
      ))}
      <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 ml-auto w-fit max-w-full">
        <div className="whitespace-pre-wrap break-all">{m.message.content}</div>
      </div>
    </>
  );
}

export function ReasoningAssistantMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}

export function ReasoningFunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}

export function ReasoningMessages({
  reasoningMessages,
  i,
  toggleReasoning,
  showReasoning,
}: {
  reasoningMessages: ChatMessage[];
  i: number;
  toggleReasoning: (index: number) => void;
  showReasoning: boolean;
}) {
  if (reasoningMessages.length === 0) return null;

  return (
    <div className="mt-4 -mb-4">
      <Button
        variant="ghost"
        onClick={() => {
          toggleReasoning(i);
        }}
      >
        {showReasoning ? "Hide" : "Show"} Thinking
        <ChevronDown
          className={`${showReasoning ? "rotate-180" : ""} transition-transform`}
        />
      </Button>
      <div
        className={`grid transition-[grid-template-rows] duration-500 ${showReasoning ? "grid-rows-[1fr]" : "grid-rows-[0fr]"}`}
      >
        <div className="overflow-hidden">
          <div className="flex flex-row mt-2">
            <div className="w-[1px] bg-black/40 rounded-full ml-6 mr-4 flex-shrink-0" />
            <div className="flex-1 space-y-4">
              {reasoningMessages.map((rm) => (
                <div key={rm.message.id}>
                  {rm.message.role.toLowerCase() === "assistant" ? (
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
}

export function AssistantMessage({
  messages,
  m,
  i,
  openSecondary,
  toggleReasoning,
  showReasoning,
}: {
  messages: ChatMessage[];
  m: ChatMessage;
  i: number;
  openSecondary: (dest: string) => void;
  toggleReasoning: (index: number) => void;
  showReasoning: boolean;
}) {
  return (
    <div className="overflow-hidden">
      {/* Reasoning Messages */}
      <ReasoningMessages
        reasoningMessages={m.reasoningMessages}
        i={i}
        toggleReasoning={toggleReasoning}
        showReasoning={showReasoning}
      />
      <div className="h-4 flex items-center justify-start -mb-2">
        {hasPreviousMemory(messages, i) ? (
          <HoverCard>
            <HoverCardTrigger asChild>
              <div className="flex items-center gap-1 text-xs text-black/50">
                <NotebookPen className="h-4 w-4" />
                <span className="font-bold">Updated saved memory</span>
              </div>
            </HoverCardTrigger>
            <HoverCardContent side="top" className="w-min max-w-80 bg-white/70">
              <div className="space-y-3">
                <div>
                  <p className="text-sm text-black">
                    {messages[i - 1]?.memory?.text ||
                      "No memory text available"}
                  </p>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full bg-white/50"
                  onClick={(e) => {
                    e.preventDefault();
                    openSecondary("memories");
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
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}

export function FunctionMessage({ m }: { m: ChatMessage }) {
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}
