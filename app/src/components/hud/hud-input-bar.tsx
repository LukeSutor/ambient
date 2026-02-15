"use client";

import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
} from "@/components/ui/input-group";
import { useConversation } from "@/lib/conversations";
import { useModelAccess } from "@/lib/model-access";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { useWindows } from "@/lib/windows/useWindows";
import type { HudDimensions } from "@/types/settings";
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ArrowUpIcon, Globe, Square, X } from "lucide-react";
import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import TextareaAutosize from "react-textarea-autosize";
import { toast } from "sonner";
import { AttachmentList } from "./attachment-list";
import { ModelSelector } from "./model-selector";
import { PlusMenu } from "./plus-menu";
import { ToolMenu } from "./tool-menu";

export function HUDInputBar() {
  const [input, setInput] = useState("");

  const inputRef = useRef<HTMLDivElement | null>(null);
  const dimensionsRef = useRef<HudDimensions | null>(null);

  // Dropdown open states
  const [isPlusDropdownOpen, setIsPlusDropdownOpen] = useState(false);
  const [isToolsDropdownOpen, setIsToolsDropdownOpen] = useState(false);
  const [isModelDropdownOpen, setIsModelDropdownOpen] = useState(false);

  // Dynamic spacer height — measured from dropdown content
  const [spacerHeight, setSpacerHeight] = useState(0);
  const spacerTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    // Clear any pending retraction timeout
    if (spacerTimeoutRef.current) {
      clearTimeout(spacerTimeoutRef.current);
      spacerTimeoutRef.current = null;
    }

    const isOpen = isPlusDropdownOpen || isToolsDropdownOpen || isModelDropdownOpen;
    if (!isOpen) {
      // Delay retraction so the dropdown close animation can play
      spacerTimeoutRef.current = setTimeout(() => setSpacerHeight(0), 150);
      return () => {
        if (spacerTimeoutRef.current) {
          clearTimeout(spacerTimeoutRef.current);
          spacerTimeoutRef.current = null;
        }
      };
    }

    // Measure dropdown content after Radix renders it
    const rAF = requestAnimationFrame(() => {
      const popperWrapper = document.querySelector<HTMLElement>(
        "[data-radix-popper-content-wrapper]",
      );
      if (popperWrapper) {
        // Use the wrapper's full height + sideOffset buffer
        setSpacerHeight(popperWrapper.offsetHeight + 5);
      }
    });

    return () => cancelAnimationFrame(rAF);
  }, [isPlusDropdownOpen, isToolsDropdownOpen, isModelDropdownOpen]);

  const {
    ocrLoading,
    isStreaming,
    conversationType,
    conversationId,
    addAttachmentData,
    toggleBrowserUse,
    sendMessage,
    stopGeneration,
  } = useConversation();
  const { closeHUD, setChatExpanded } = useWindows();
  const { hudDimensions, settings } = useSettings();
  const { getUsage, enabledModels } = useModelAccess();

  // Computed values
  const isLoading = ocrLoading || isStreaming;
  const isBrowserUseActive = conversationType === "browser_use";

  // Memoized styles
  const containerStyle = useMemo(
    () => ({
      minHeight: hudDimensions ? `${hudDimensions.input_bar_height}px` : "60px",
      width: hudDimensions ? `${hudDimensions.chat_width}px` : "500px",
      opacity: hudDimensions ? 1 : 0,
      transform: hudDimensions ? "scale(1)" : "scale(0)",
    }),
    [hudDimensions],
  );

  const handleUploadFiles = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const files = e.target.files;
      if (!files) return;

      for (const file of Array.from(files)) {
        const reader = new FileReader();
        reader.onload = () => {
          if (reader.result) {
            addAttachmentData({
              name: file.name,
              file_type: file.type,
              data: reader.result as string,
            });
          }
        };
        reader.readAsDataURL(file);
      }
    },
    [addAttachmentData],
  );

  const handleSubmit = useCallback(async () => {
    const query = input.trim();
    if (!query || isLoading) return;

    // Pre-check: block send if the selected cloud model is at its usage limit
    const modelId = settings?.model_selection;
    if (modelId) {
      const selectedModel = enabledModels.find((m) => m.id.toString() === modelId);
      const usage = selectedModel ? getUsage(selectedModel.model) : undefined;
      if (usage) {
        if (!usage.is_available) {
          toast.error("This model is not available on your current plan. Please upgrade or switch models.");
          return;
        }
        if (usage.remaining === 0 && usage.daily_limit !== -1) {
          toast.error("You've reached your daily usage limit for this model. Try again tomorrow or switch to a different model.");
          return;
        }
      }
    }

    setChatExpanded();
    setInput("");

    try {
      await sendMessage(conversationId, query);
    } catch (error) {
      console.error("Error in handleSubmit:", error);
    }
  }, [input, isLoading, conversationId, sendMessage, setChatExpanded, settings, getUsage, enabledModels]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && !e.shiftKey && !e.altKey && !e.ctrlKey) {
        e.preventDefault();
        void handleSubmit();
      }
    },
    [handleSubmit],
  );

  const handleToggleBrowserUse = useCallback(() => {
    toggleBrowserUse();
  }, [toggleBrowserUse]);

  const handleCloseWindow = useCallback(() => {
    void closeHUD();
  }, [closeHUD]);

  const onStopGeneration = useCallback(() => {
    void stopGeneration();
  }, [stopGeneration]);

  // Input bar enter animation
  useGSAP(() => {
    // Skip if dimensions haven't changed
    if (
      dimensionsRef.current &&
      hudDimensions &&
      JSON.stringify(dimensionsRef.current) === JSON.stringify(hudDimensions)
    ) {
      return;
    }
    dimensionsRef.current = hudDimensions;

    if (hudDimensions && inputRef.current) {
      gsap.fromTo(
        inputRef.current,
        { scale: 0.9, opacity: 0, transformOrigin: "center center" },
        {
          scale: 1,
          opacity: 1,
          duration: 0.2,
          ease: "power2.out",
          delay: 0.1,
        },
      );
    }
  }, [hudDimensions]);

  return (
    <div
      className="flex flex-col justify-start items-center relative"
      ref={inputRef}
      style={containerStyle}
    >
      <InputGroup
        data-tauri-drag-region
        className={cn(
          "bg-white/60 border border-black/20 transition-all rounded-md flex-col items-stretch relative cursor-grab active:cursor-grabbing streaming-ring",
          "has-[[data-slot=input-group-control]:focus-visible]:ring-0 has-[[data-slot=input-group-control]:focus-visible]:border-black/20",
          isStreaming &&
            "streaming-active border-transparent has-[[data-slot=input-group-control]:focus-visible]:border-transparent",
        )}
      >
        {/* Close button — subtle, top-right corner */}
        <button
          type="button"
          className="absolute top-1 right-1 z-10 flex items-center justify-center w-5 h-5 rounded-sm text-black/40 hover:text-black/60 hover:bg-black/5 transition-colors"
          onClick={handleCloseWindow}
          title="Close Window"
        >
          <X className="w-4 h-4" />
        </button>

        <AttachmentList />

        {/* Textarea wrapper — side padding is drag region, top padding is drag region */}
        <div
          data-tauri-drag-region
          className="px-3 pt-2 select-none"
        >
          <TextareaAutosize
            data-slot="input-group-control"
            maxRows={4}
            minRows={2}
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
            }}
            onKeyDown={handleKeyDown}
            className="flex field-sizing-content hud-scroll min-h-16 w-full resize-none rounded-md bg-transparent px-0 pt-0 pb-2 text-base transition-[color,box-shadow] outline-none cursor-text md:text-sm"
            placeholder="Ask anything"
            autoComplete="off"
            autoFocus
          />
        </div>

        <InputGroupAddon
          data-tauri-drag-region
          align="block-end"
          className="flex items-center gap-1.5 px-3 pb-2 pt-0 cursor-grab active:cursor-grabbing select-none"
        >
          <PlusMenu
            onOpenChange={setIsPlusDropdownOpen}
            disabled={isLoading}
            handleUploadFiles={handleUploadFiles}
          />

          <ToolMenu
            onOpenChange={setIsToolsDropdownOpen}
            disabled={isLoading}
          />

          {isBrowserUseActive && (
            <div className="flex items-center justify-center bg-blue-500/30 rounded-xl px-2 py-1 shrink-0 overflow-hidden whitespace-nowrap transition-all duration-150">
              <Globe className="!h-4 !w-4 text-black" />
              <p className="mx-1 text-black text-xs font-medium">
                Browser Use
              </p>
              <Button
                variant="ghost"
                className="!h-4 !w-4 text-black shrink-0 hover:bg-transparent p-0"
                size="icon"
                onClick={handleToggleBrowserUse}
              >
                <X className="!h-3 !w-3 text-black shrink-0" />
              </Button>
            </div>
          )}

          <ModelSelector
            onOpenChange={setIsModelDropdownOpen}
            disabled={isLoading}
          />

          {isStreaming ? (
            <InputGroupButton
              variant="ghost"
              className="rounded-full hover:bg-red-50 text-black/80 hover:text-red-600 transition-colors"
              size="icon-xs"
              type="button"
              onClick={onStopGeneration}
              title="Stop generation"
            >
              <Square className="!h-3 !w-3 fill-current" />
              <span className="sr-only">Stop</span>
            </InputGroupButton>
          ) : (
            <InputGroupButton
              variant="default"
              className="rounded-full bg-black/80 hover:bg-black"
              size="icon-xs"
              type="submit"
              onClick={() => {
                void handleSubmit();
              }}
              disabled={ocrLoading || !input.trim()}
            >
              <ArrowUpIcon />
              <span className="sr-only">Send</span>
            </InputGroupButton>
          )}
        </InputGroupAddon>

      </InputGroup>

      {/* Spacer to expand window when dropdowns are open */}
      <div
        className="pointer-events-none overflow-hidden"
        style={{ height: spacerHeight }}
      />
    </div>
  );
}

export default HUDInputBar;
