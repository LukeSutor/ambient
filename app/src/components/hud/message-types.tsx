"use client";

import {
  llmMarkdownConfig,
  preprocessMarkdownCurrency,
} from "@/components/ui/markdown-config";
import { type ChatMessage, useConversation } from "@/lib/conversations";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import type { Attachment, MessageMetadata } from "@/types/conversations";
import { convertFileSrc } from "@tauri-apps/api/core";
import { appDataDir, join } from "@tauri-apps/api/path";
import {
  Camera,
  Check,
  CheckCircle,
  CheckCircle2,
  ChevronDown,
  Copy,
  FileText,
  Hammer,
  Loader2,
  NotebookPen,
  Pencil,
  RefreshCw,
  Search,
  Sparkles,
  SquareDashed,
  X,
  XCircle,
} from "lucide-react";
import Image from "next/image";
import { memo, useCallback, useEffect, useMemo, useState } from "react";
import Markdown from "react-markdown";
import TextareaAutosize from "react-textarea-autosize";
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
import { Tooltip, TooltipContent, TooltipTrigger } from "../ui/tooltip";

export const PreviewAttachment = memo(function PreviewAttachment({
  a,
  variant = "default",
}: {
  a: Attachment;
  variant?: "default" | "small";
}) {
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
    const isSmall = variant === "small";
    return (
      <div
        className={cn(
          "my-2",
          variant === "default" ? "max-w-[80%] ml-auto" : "max-w-[200px] ml-0",
        )}
      >
        <Dialog>
          <DialogTrigger asChild>
            <button
              type="button"
              className="relative w-full group outline-none text-left"
            >
              <Image
                src={fileSrc}
                alt={a.file_name}
                className={cn(
                  "h-auto rounded-lg transition-all group-hover:brightness-75 border border-black/5 shadow-sm",
                  isSmall ? "w-full" : "w-full",
                )}
                width={isSmall ? 200 : 400}
                height={isSmall ? 200 : 400}
                unoptimized
              />
              <div className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none">
                <Search
                  className={cn(
                    "text-white drop-shadow-md",
                    isSmall ? "w-4 h-4" : "w-8 h-8",
                  )}
                />
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
});

export const UserMessage = memo(function UserMessage({
  m,
  openSecondary,
}: {
  m: ChatMessage;
  openSecondary: (dest: string) => void;
}) {
  const { resubmitMessage, isStreaming } = useConversation();
  const [isEditing, setIsEditing] = useState(false);
  const [editContent, setEditContent] = useState("");

  useEffect(() => {
    // Set the content when the message loads or changes
    setEditContent(m.message.content);
  }, [m.message.content]);

  const handleCopy = () => {
    void navigator.clipboard.writeText(m.message.content);
  };

  const handleUpdate = () => {
    if (editContent.trim() && editContent !== m.message.content) {
      void resubmitMessage(m.message.id, editContent);
      setIsEditing(false);
    }
  };

  const handleCancel = () => {
    setEditContent(m.message.content);
    setIsEditing(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && !e.altKey && !e.ctrlKey) {
      e.preventDefault();
      handleUpdate();
    }
  };

  return (
    <>
      {m.message.attachments.map((a) => (
        <PreviewAttachment key={a.id} a={a} />
      ))}
      <div className="flex items-start gap-1 justify-end group">
        <div className="w-14">
          {!isEditing && !isStreaming && (
            <div className="flex flex-row opacity-0 group-hover:opacity-100 transition-opacity">
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    onClick={handleCopy}
                    variant="ghost"
                    size="icon"
                    className="rounded-full h-7 w-7"
                  >
                    <Copy className="!w-3 !h-3" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Copy</TooltipContent>
              </Tooltip>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    onClick={() => {
                      setIsEditing(true);
                    }}
                    variant="ghost"
                    size="icon"
                    className="rounded-full h-7 w-7"
                  >
                    <Pencil className="!w-3 !h-3" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent side="bottom">Edit</TooltipContent>
              </Tooltip>
            </div>
          )}
        </div>

        {isEditing ? (
          <div className="flex flex-col gap-2 w-full">
            <TextareaAutosize
              minRows={1}
              maxRows={10}
              value={editContent}
              onChange={(e) => {
                setEditContent(e.target.value);
              }}
              onKeyDown={handleKeyDown}
              className="w-full bg-white/80 border border-black/20 rounded-lg px-3 py-2 text-sm focus:outline-none focus:ring-1 focus:ring-black/20 resize-none"
              autoFocus
            />
            <div className="flex justify-end gap-2 px-1">
              <Button
                variant="ghost"
                size="sm"
                onClick={handleCancel}
                className="h-7 text-xs font-semibold text-zinc-500 hover:text-zinc-700 hover:bg-black/5"
              >
                Cancel
              </Button>
              <Button
                variant="default"
                size="sm"
                onClick={handleUpdate}
                className="h-7 text-xs font-bold bg-zinc-900 text-white hover:bg-zinc-800"
                disabled={
                  !editContent.trim() ||
                  editContent.trim() === m.message.content
                }
              >
                Update
              </Button>
            </div>
          </div>
        ) : (
          <div className="overflow-hidden bg-white/60 border border-black/20 rounded-lg px-3 py-2 w-fit max-w-[85%]">
            <div className="whitespace-pre-wrap break-all text-sm leading-relaxed text-zinc-800">
              {m.message.content}
            </div>
          </div>
        )}
      </div>

      {/* Persistent memory area to avoid layout shifts and provide spacing */}
      <div className="h-10 flex items-end justify-start">
        {m.message.memory && (
          <div className="mb-2 ml-1">
            <HoverCard>
              <HoverCardTrigger asChild>
                <div className="flex items-center gap-1 text-xs text-zinc-600 cursor-pointer hover:text-zinc-800 transition-colors">
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
});

export const ToolStep = memo(function ToolStep({
  call,
  result,
  attachments,
}: {
  call: MessageMetadata;
  result?: MessageMetadata;
  attachments?: Attachment[];
}) {
  if (call.type !== "ToolCall") return null;

  const resultMetadata = result?.type === "ToolResult" ? result : null;
  const isSuccess = resultMetadata?.success ?? null;

  // Get screenshot attachment from result if it exists
  const screenshotId = resultMetadata?.screenshot_attachment_id;
  const screenshotAttachment = attachments?.find(
    (a) =>
      (screenshotId && a.id === screenshotId) ||
      (!screenshotId && a.file_type.startsWith("image/")),
  );

  return (
    <div className="flex flex-col gap-1.5 py-2 border-b border-zinc-50 last:border-0 first:pt-0">
      <div className="flex items-center gap-2 text-zinc-700">
        <div className="p-1 rounded bg-zinc-100 shadow-sm border border-zinc-200/50">
          <Hammer className="w-3.5 h-3.5 text-zinc-500" />
        </div>
        <span className="text-sm font-semibold tracking-tight">
          {call.skill_name}.{call.tool_name}
        </span>
        {isSuccess === true && (
          <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
        )}
        {isSuccess === false && (
          <XCircle className="w-3.5 h-3.5 text-red-500" />
        )}
        {isSuccess === null && (
          <div className="flex gap-0.5">
            <div className="w-1 h-1 rounded-full bg-zinc-400 animate-bounce" />
            <div className="w-1 h-1 rounded-full bg-zinc-400 animate-bounce [animation-delay:0.2s]" />
            <div className="w-1 h-1 rounded-full bg-zinc-400 animate-bounce [animation-delay:0.4s]" />
          </div>
        )}
      </div>

      <div className="ml-7 text-xs text-zinc-500 font-mono bg-zinc-50/80 p-2.5 rounded-lg border border-zinc-100 overflow-x-auto shadow-inner">
        <pre className="whitespace-pre-wrap break-all">
          {JSON.stringify(call.arguments, null, 2)}
        </pre>
      </div>

      {resultMetadata && (
        <div className="ml-7 mt-1 space-y-2">
          {resultMetadata.result && (
            <div className="text-xs text-zinc-600 bg-white p-2.5 rounded-lg border border-zinc-100 shadow-sm relative overflow-hidden">
              <div className="absolute top-0 left-0 w-1 h-full bg-zinc-200/50" />
              <div className="font-bold mb-1 uppercase text-[9px] text-zinc-400 tracking-widest pl-2">
                Output
              </div>
              <pre className="whitespace-pre-wrap break-all pl-2">
                {typeof resultMetadata.result === "string"
                  ? resultMetadata.result
                  : JSON.stringify(resultMetadata.result, null, 2)}
              </pre>
            </div>
          )}

          {resultMetadata.error && (
            <div className="text-xs text-red-600 bg-red-50/50 p-2.5 rounded-lg border border-red-100 font-mono shadow-sm">
              <div className="font-bold mb-1 uppercase text-[9px] text-red-400 tracking-widest">
                Error
              </div>
              {resultMetadata.error}
            </div>
          )}

          {screenshotAttachment && (
            <div className="mt-2">
              <PreviewAttachment a={screenshotAttachment} variant="small" />
            </div>
          )}
        </div>
      )}
    </div>
  );
});

export const GenericThinkingStep = memo(function GenericThinkingStep({
  m,
}: { m: ChatMessage }) {
  if (!m.message.content) return null;
  return (
    <div className="flex flex-col gap-1.5 py-1">
      <div className="flex items-center gap-2 text-zinc-400">
        <Sparkles className="w-3 h-3" />
        <span className="text-[10px] items-center font-bold uppercase tracking-wider">
          Thought
        </span>
      </div>
      <div className="ml-5 text-sm text-zinc-600 font-medium border-l-2 border-zinc-100 pl-3 py-0.5 mb-2">
        <Markdown {...llmMarkdownConfig}>
          {preprocessMarkdownCurrency(m.message.content)}
        </Markdown>
      </div>
    </div>
  );
});

type FriendlyToolMessage = {
  loadingPrefix: string;
  finishedPrefix: string;
  suffix: string;
};
function getFriendlyToolMessage(call: MessageMetadata): FriendlyToolMessage {
  if (call.type !== "ToolCall")
    return {
      loadingPrefix: "Using",
      finishedPrefix: "Used",
      suffix: "a tool",
    };
  /* eslint-disable @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call */
  const { skill_name, tool_name, arguments: args } = call;
  switch (`${skill_name}.${tool_name}`) {
    case "web-search.search_web":
      return {
        loadingPrefix: "Searching",
        finishedPrefix: "Searched",
        suffix: `the web for "${args.query}"`,
      };
    case "web-search.fetch_webpage":
      return {
        loadingPrefix: "Fetching",
        finishedPrefix: "Fetched",
        suffix: `webpage: ${args.url}`,
      };
    case "code-execution.execute_code":
      return {
        loadingPrefix: "Executing",
        finishedPrefix: "Executed",
        suffix: "code",
      };
    case "calendar.create_event":
      return {
        loadingPrefix: "Creating",
        finishedPrefix: "Created",
        suffix: "calendar event",
      };
    case "calendar.list_events":
      return {
        loadingPrefix: "Checking",
        finishedPrefix: "Checked",
        suffix: "calendar",
      };
    case "email.send_email":
      return {
        loadingPrefix: "Sending",
        finishedPrefix: "Sent",
        suffix: "email",
      };
    case "email.list_emails":
      return {
        loadingPrefix: "Fetching",
        finishedPrefix: "Fetched",
        suffix: "emails",
      };
    case "memory-search.search_memories":
      return {
        loadingPrefix: "Searching",
        finishedPrefix: "Searched",
        suffix: `memories for "${args.query}"`,
      };
    case "system.activate_skill":
      return {
        loadingPrefix: "Loading",
        finishedPrefix: "Loaded",
        suffix: `${args.skill_name.replace("-", " ")} tools`,
      };
    case "automation-management.list_automations":
      return {
        loadingPrefix: "Listing",
        finishedPrefix: "Listed",
        suffix: "automations",
      };
    case "automation-management.create_automation":
      return {
        loadingPrefix: "Creating",
        finishedPrefix: "Created",
        suffix: "an automation",
      };
    case "automation-management.run_automation":
      return {
        loadingPrefix: "Running",
        finishedPrefix: "Ran",
        suffix: "an automation",
      };
    case "browser.navigate":
      return {
        loadingPrefix: "Navigating",
        finishedPrefix: "Navigated",
        suffix: `to ${args.url}`,
      };
    case "browser.click":
      return {
        loadingPrefix: "Clicking",
        finishedPrefix: "Clicked",
        suffix: "on element",
      };
    case "browser.type_text":
      return {
        loadingPrefix: "Typing",
        finishedPrefix: "Typed",
        suffix: `text "${args.text}"`,
      };
    case "browser.select_option":
      return {
        loadingPrefix: "Selecting",
        finishedPrefix: "Selected",
        suffix: "dropdown option",
      };
    case "browser.scroll":
      return {
        loadingPrefix: "Scrolling",
        finishedPrefix: "Scrolled",
        suffix: `the page ${args.direction}`,
      };
    case "browser.go_back":
      return {
        loadingPrefix: "Going back",
        finishedPrefix: "Went back",
        suffix: "",
      };
    case "browser.wait":
      return {
        loadingPrefix: "Waiting",
        finishedPrefix: "Waited",
        suffix: `for ${args.duration} seconds`,
      };
    default:
      return {
        loadingPrefix: "Using",
        finishedPrefix: "Used",
        suffix: tool_name,
      };
  }
  /* eslint-enable @typescript-eslint/no-unsafe-assignment, @typescript-eslint/no-unsafe-member-access, @typescript-eslint/no-unsafe-call */
}

export const SimplifiedToolStep = memo(function SimplifiedToolStep({
  call,
  result,
}: {
  call: MessageMetadata;
  result?: MessageMetadata;
}) {
  if (call.type !== "ToolCall") return null;

  const resultMetadata = result?.type === "ToolResult" ? result : null;
  const isSuccess = resultMetadata?.success ?? null;
  const friendlyMessage = getFriendlyToolMessage(call);

  return (
    <div className="flex items-center gap-2 py-1 text-zinc-600">
      <div className="relative flex items-center justify-center">
        {isSuccess === null ? (
          <Loader2 className="w-3.5 h-3.5 animate-spin" />
        ) : isSuccess ? (
          <Check className="w-3.5 h-3.5" />
        ) : (
          <X className="w-3.5 h-3.5 text-red-500" />
        )}
      </div>
      <span className="text-xs font-medium transition-colors duration-200">
        {isSuccess === null
          ? friendlyMessage.loadingPrefix
          : friendlyMessage.finishedPrefix}{" "}
        {friendlyMessage.suffix}
      </span>
    </div>
  );
});

export const ThinkingBlock = memo(function ThinkingBlock({
  messages,
  isExpanded,
  onToggle,
}: {
  messages: ChatMessage[];
  isExpanded: boolean;
  onToggle: () => void;
}) {
  const { settings } = useSettings();
  const showFullThoughtTraces = settings?.show_full_thought_traces ?? false;
  // Map call_id -> { message, metadata }
  const resultsMap = useMemo(() => {
    const map = new Map<
      string,
      { message: ChatMessage; metadata: MessageMetadata }
    >();
    if (messages.length === 0) return map;
    for (const m of messages) {
      const metadata = m.message.metadata;
      if (Array.isArray(metadata)) {
        for (const meta of metadata) {
          if (meta.type === "ToolResult") {
            map.set(meta.call_id, { message: m, metadata: meta });
          }
        }
      }
    }
    return map;
  }, [messages]);

  if (messages.length === 0) return null;

  if (!showFullThoughtTraces) {
    return (
      <div className="flex flex-col space-y-1">
        {messages.map((m) => {
          const metadataList = m.message.metadata;
          return (
            <div key={m.message.id}>
              {Array.isArray(metadataList) &&
                metadataList.map((meta, idx) => {
                  if (meta.type === "ToolCall") {
                    const resultObj = resultsMap.get(meta.call_id);
                    return (
                      <SimplifiedToolStep
                        key={`${m.message.id}-${idx}`}
                        call={meta}
                        result={resultObj?.metadata}
                      />
                    );
                  }
                  return null;
                })}
            </div>
          );
        })}
      </div>
    );
  }

  return (
    <div className="flex flex-col mb-4">
      <Button
        variant="ghost"
        size="sm"
        onClick={onToggle}
        className="w-fit text-zinc-600 hover:text-zinc-700 hover:bg-zinc-100 h-8 px-2 -ml-2 transition-colors flex items-center gap-1.5 rounded-full"
      >
        <div className="p-0.5 rounded-full bg-zinc-50 border border-zinc-200">
          <Sparkles className="w-3 h-3 text-zinc-400" />
        </div>
        <span className="text-[10px] font-bold uppercase tracking-widest">
          {isExpanded ? "Hide" : "Show"} Thinking
        </span>
        <ChevronDown
          className={cn(
            "w-3 h-3 transition-transform duration-300",
            isExpanded && "rotate-180",
          )}
        />
      </Button>

      <div
        className={cn(
          "grid transition-all duration-500 ease-in-out overflow-hidden",
          isExpanded
            ? "grid-rows-[1fr] opacity-100 mt-3"
            : "grid-rows-[0fr] opacity-0",
        )}
      >
        <div
          className={cn(
            "min-h-0 bg-zinc-50/30 rounded-xl border border-dashed border-zinc-200 ml-1 transition-padding duration-[0]",
            isExpanded ? "p-3 delay-0" : "p-0 delay-300",
          )}
        >
          <div className="ml-2 border-l-2 border-zinc-100/50 pl-4 space-y-4">
            {messages.map((m) => {
              const metadataList = m.message.metadata;
              const content = m.message.content;

              return (
                <div key={m.message.id}>
                  {content && <GenericThinkingStep m={m} />}
                  {Array.isArray(metadataList) &&
                    metadataList.map((meta, idx) => {
                      if (meta.type === "ToolCall") {
                        const resultObj = resultsMap.get(meta.call_id);
                        return (
                          <ToolStep
                            key={`${m.message.id}-${idx}`}
                            call={meta}
                            result={resultObj?.metadata}
                            attachments={resultObj?.message.message.attachments}
                          />
                        );
                      }
                      return null;
                    })}
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
});

export const AssistantMessage = memo(function AssistantMessage({
  m,
}: { m: ChatMessage }) {
  const { isStreaming, retryMessage } = useConversation();
  const content = m.message.content;

  const handleCopy = () => {
    if (content) {
      void navigator.clipboard.writeText(content);
    }
  };

  const handleRetry = () => {
    void retryMessage(m.message.id);
  };

  return (
    <div className="flex flex-col space-y-1 mb-4">
      <div className="overflow-hidden">
        <Markdown {...llmMarkdownConfig}>
          {preprocessMarkdownCurrency(content)}
        </Markdown>
      </div>
      {/* Redo and copy section */}
      <div className="h-7">
        {!isStreaming && (
          <div className="flex flex-row">
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  onClick={handleCopy}
                  variant="ghost"
                  size="icon"
                  className="rounded-full h-7 w-7"
                >
                  <Copy className="!w-3 !h-3" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p>Copy</p>
              </TooltipContent>
            </Tooltip>
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  onClick={handleRetry}
                  variant="ghost"
                  size="icon"
                  className="rounded-full h-7 w-7"
                >
                  <RefreshCw className="!w-3 !h-3" />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">
                <p>Redo</p>
              </TooltipContent>
            </Tooltip>
          </div>
        )}
      </div>
    </div>
  );
});

export const FunctionMessage = memo(function FunctionMessage({
  m,
}: { m: ChatMessage }) {
  // If this is rendered outside a thinking block (fallback)
  return (
    <div className="overflow-hidden bg-white/20 border border-white/30 rounded-lg px-3 py-2 max-w-[95%] w-fit text-left mt-6">
      <Markdown {...llmMarkdownConfig}>
        {preprocessMarkdownCurrency(m.message.content)}
      </Markdown>
    </div>
  );
});
