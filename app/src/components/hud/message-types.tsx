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
  Hammer,
  NotebookPen,
  Search,
  Sparkles,
  SquareDashed,
  CheckCircle2,
  XCircle,
} from "lucide-react";
import Image from "next/image";
import { useEffect, useState } from "react";
import Markdown from "react-markdown";
import { Button } from "../ui/button";
import { cn } from "@/lib/utils";
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

export function UserMessage({
  m,
  openSecondary,
}: {
  m: ChatMessage;
  openSecondary: (dest: string) => void;
}) {
  return (
    <>
      {m.message.attachments.map((a) => (
        <PreviewAttachment key={a.id} a={a} />
      ))}
      <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 ml-auto w-fit max-w-[85%]">
        <div className="whitespace-pre-wrap break-all">{m.message.content}</div>
      </div>

      {/* Persistent memory area to avoid layout shifts and provide spacing */}
      <div className="h-10 flex items-end justify-start">
        {m.message.memory && (
          <div className="mb-1 ml-1">
            <HoverCard>
              <HoverCardTrigger asChild>
                <div className="flex items-center gap-1 text-xs text-black/50 cursor-pointer hover:text-black/70 transition-colors">
                  <NotebookPen className="h-4 w-4" />
                  <span className="font-bold">Saved memory</span>
                </div>
              </HoverCardTrigger>
              <HoverCardContent
                side="top"
                className="w-min max-w-80 bg-white/70"
              >
                <div className="space-y-3">
                  <div>
                    <p className="text-sm text-black">
                      {m.message.memory.text || "No memory text available"}
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
          </div>
        )}
      </div>
    </>
  );
}

export function ToolStep({
  call,
  result,
}: {
  call: ChatMessage;
  result?: ChatMessage;
}) {
  const metadata = call.message.metadata;
  if (metadata?.type !== "ToolCall") return null;

  const resultMetadata = result?.message.metadata;
  const isSuccess =
    resultMetadata?.type === "ToolResult" ? resultMetadata.success : null;

  return (
    <div className="flex flex-col gap-1.5 py-1">
      <div className="flex items-center gap-2 text-zinc-600">
        <div className="p-1 rounded bg-zinc-100">
          <Hammer className="w-3.5 h-3.5" />
        </div>
        <span className="text-sm font-medium">
          {metadata.skill_name}.{metadata.tool_name}
        </span>
        {isSuccess === true && (
          <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
        )}
        {isSuccess === false && <XCircle className="w-3.5 h-3.5 text-red-500" />}
      </div>

      <div className="ml-7 text-xs text-zinc-500 font-mono bg-zinc-50/50 p-2 rounded border border-zinc-100 overflow-x-auto">
        {JSON.stringify(metadata.arguments, null, 2)}
      </div>

      {result && resultMetadata?.type === "ToolResult" && resultMetadata.result && (
        <div className="ml-7 mt-1 text-xs text-zinc-600 bg-white p-2 rounded border border-zinc-100 shadow-sm">
          <div className="font-semibold mb-1 uppercase text-[10px] text-zinc-400 tracking-wider">
            Result
          </div>
          <pre className="whitespace-pre-wrap break-all">
            {typeof resultMetadata.result === "string"
              ? resultMetadata.result
              : JSON.stringify(resultMetadata.result, null, 2)}
          </pre>
        </div>
      )}

      {result &&
        resultMetadata?.type === "ToolResult" &&
        resultMetadata.error && (
          <div className="ml-7 mt-1 text-xs text-red-600 bg-red-50 p-2 rounded border border-red-100 font-mono">
            {resultMetadata.error}
          </div>
        )}
    </div>
  );
}

export function GenericThinkingStep({ m }: { m: ChatMessage }) {
  return (
    <div className="flex items-center gap-2 text-zinc-500 py-1">
      <div className="p-1 rounded bg-zinc-100">
        <Sparkles className="w-3.5 h-3.5" />
      </div>
      <span className="text-sm">{m.message.content}</span>
    </div>
  );
}

export function ThinkingBlock({
  messages,
  isExpanded,
  onToggle,
}: {
  messages: ChatMessage[];
  isExpanded: boolean;
  onToggle: () => void;
}) {
  if (messages.length === 0) return null;

  const resultsMap = new Map();
  for (const m of messages) {
    const mType = (m.message.message_type || "").toLowerCase();
    if (
      (mType === "tool_result" || mType === "toolresult") &&
      m.message.metadata?.type === "ToolResult"
    ) {
      resultsMap.set(m.message.metadata.call_id, m);
    }
  }

  return (
    <div className="flex flex-col mb-4">
      <Button
        variant="ghost"
        size="sm"
        onClick={onToggle}
        className="w-fit text-zinc-400 hover:text-zinc-600 hover:bg-zinc-100 h-8 px-2 -ml-2 transition-colors flex items-center gap-1.5"
      >
        <span className="text-xs font-semibold uppercase tracking-wider">
          {isExpanded ? "Hide" : "Show"} Thinking
        </span>
        <ChevronDown
          className={cn(
            "w-3.5 h-3.5 transition-transform duration-200",
            isExpanded && "rotate-180",
          )}
        />
      </Button>

      <div
        className={cn(
          "grid transition-all duration-300 ease-in-out overflow-hidden",
          isExpanded
            ? "grid-rows-[1fr] opacity-100 mt-2"
            : "grid-rows-[0fr] opacity-0",
        )}
      >
        <div className="min-h-0">
          <div className="ml-2 border-l-2 border-zinc-100 pl-4 space-y-1">
            {messages.map((m) => {
              const mType = (m.message.message_type || "").toLowerCase();
              const role = m.message.role.toLowerCase();

              if (mType === "tool_call" || mType === "toolcall") {
                const result = resultsMap.get(
                  m.message.metadata?.type === "ToolCall"
                    ? m.message.metadata.call_id
                    : "",
                );
                return <ToolStep key={m.message.id} call={m} result={result} />;
              }

              // Only render individual steps for non-result messages
              // (Results are rendered inside ToolStep)
              if (
                mType !== "tool_result" &&
                mType !== "toolresult" &&
                role !== "tool"
              ) {
                return <GenericThinkingStep key={m.message.id} m={m} />;
              }

              return null;
            })}
          </div>
        </div>
      </div>
    </div>
  );
}

export function AssistantMessage({ m }: { m: ChatMessage }) {
  if (!m.message.content) return null;

  return (
    <div className="overflow-hidden">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}

export function FunctionMessage({ m }: { m: ChatMessage }) {
  // If this is rendered outside a thinking block (fallback)
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
}
