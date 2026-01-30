"use client";

import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
} from "@/components/ui/input-group";
import { useConversation } from "@/lib/conversations";
import { cn } from "@/lib/utils";
import { useWindows } from "@/lib/windows/useWindows";
import type { HudDimensions } from "@/types/settings";
import { useGSAP } from "@gsap/react";
import gsap from "gsap";
import { ArrowUpIcon, MousePointerClick, Move, Square, X } from "lucide-react";
import type React from "react";
import { useCallback, useMemo, useRef, useState } from "react";
import TextareaAutosize from "react-textarea-autosize";
import { AttachmentList } from "./attachment-list";
import { ModelSelector } from "./model-selector";
import { PlusMenu } from "./plus-menu";
import { ToolMenu } from "./tool-menu";

interface HUDInputBarProps {
  hudDimensions: HudDimensions | null;
  inputValue: string;
  setInputValue: (v: string) => void;
  handleSubmit: () => Promise<void>;
  onKeyDown: (
    e: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>,
  ) => void;
  onDragStart: () => void;
  onMouseLeave: (e: React.MouseEvent) => void;
  isDraggingWindow: boolean;
  isHoveringGroup: boolean;
  setIsHoveringGroup: (b: boolean) => void;
}

export function HUDInputBar({
  hudDimensions,
  inputValue,
  setInputValue,
  handleSubmit,
  onKeyDown,
  onDragStart,
  onMouseLeave,
  isDraggingWindow,
  isHoveringGroup,
  setIsHoveringGroup,
}: HUDInputBarProps) {
  const inputRef = useRef<HTMLDivElement | null>(null);
  const dimensionsRef = useRef<HudDimensions | null>(null);

  // Dropdown open states
  const [isPlusDropdownOpen, setIsPlusDropdownOpen] = useState(false);
  const [isToolsDropdownOpen, setIsToolsDropdownOpen] = useState(false);
  const [isModelDropdownOpen, setIsModelDropdownOpen] = useState(false);

  const {
    ocrLoading,
    isStreaming,
    conversationType,
    addAttachmentData,
    toggleComputerUse,
    stopGeneration,
  } = useConversation();
  const { closeHUD } = useWindows();

  // Computed values
  const isLoading = ocrLoading || isStreaming;
  const showWindowControls = isDraggingWindow || isHoveringGroup;
  const isComputerUseActive = conversationType === "computer_use";

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

  // Memoized handlers
  const handleMouseEnter = useCallback(() => {
    setIsHoveringGroup(true);
  }, [setIsHoveringGroup]);

  const handleInputChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      setInputValue(e.target.value);
    },
    [setInputValue],
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

  const handleToggleComputerUse = useCallback(() => {
    toggleComputerUse();
  }, [toggleComputerUse]);

  const handleCloseWindow = useCallback(() => {
    void closeHUD();
  }, [closeHUD]);

  const onSubmit = useCallback(() => {
    void handleSubmit();
  }, [handleSubmit]);

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
        { scale: 0, opacity: 0, transformOrigin: "center center" },
        {
          scale: 1,
          opacity: 1,
          duration: 0.25,
          ease: "back.out(0.8)",
          delay: 0.1,
        },
      );
    }
  }, [hudDimensions]);

  return (
    <div
      className="flex flex-col justify-start items-center relative p-2"
      onMouseEnter={handleMouseEnter}
      onMouseLeave={onMouseLeave}
      ref={inputRef}
      style={containerStyle}
    >
      <InputGroup
        className={cn(
          "bg-white/60 border border-black/20 transition-all rounded-md flex-col items-stretch",
          "has-[[data-slot=input-group-control]:focus-visible]:ring-0 has-[[data-slot=input-group-control]:focus-visible]:border-black/20",
          isStreaming &&
            "streaming-ring border-transparent has-[[data-slot=input-group-control]:focus-visible]:border-transparent",
        )}
      >
        <AttachmentList />

        <TextareaAutosize
          data-slot="input-group-control"
          maxRows={4}
          minRows={2}
          value={inputValue}
          onChange={handleInputChange}
          onKeyDown={onKeyDown}
          className="flex field-sizing-content hud-scroll min-h-16 w-full resize-none rounded-md bg-transparent px-3 py-2.5 text-base transition-[color,box-shadow] outline-none md:text-sm"
          placeholder="Ask anything"
          autoComplete="off"
          autoFocus
        />

        <InputGroupAddon
          align="block-end"
          className="flex items-center gap-1.5 px-3 pb-2.5 pt-0"
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

          {isComputerUseActive && (
            <div className="flex items-center justify-center bg-yellow-500/30 rounded-xl px-2 py-1 shrink-0 overflow-hidden whitespace-nowrap transition-all duration-150">
              <MousePointerClick className="!h-4 !w-4 text-black" />
              <p className="mx-1 text-black text-xs font-medium">
                Computer Use
              </p>
              <Button
                variant="ghost"
                className="!h-4 !w-4 text-black shrink-0 hover:bg-transparent p-0"
                size="icon"
                onClick={handleToggleComputerUse}
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
              onClick={onSubmit}
              disabled={ocrLoading || !inputValue.trim()}
            >
              <ArrowUpIcon />
              <span className="sr-only">Send</span>
            </InputGroupButton>
          )}
        </InputGroupAddon>

        {/* Window Controls */}
        <button
          type="button"
          className={cn(
            "absolute -top-1.5 -right-1.5 w-6 h-6 rounded-full bg-white/60 hover:bg-white/80 border border-black/20 transition-all duration-100 select-none",
            showWindowControls ? "scale-100 opacity-100" : "scale-0 opacity-0",
          )}
          onClick={handleCloseWindow}
          title="Close Window"
        >
          <X className="w-full h-full p-1 text-black pointer-events-none" />
        </button>

        <div
          data-tauri-drag-region
          id="drag-area"
          className={cn(
            "hover:cursor-grab select-none absolute -bottom-1.5 -right-1.5 w-6 h-6 bg-white/60 hover:bg-white/80 border border-black/20 rounded-full transition-all duration-100",
            showWindowControls ? "scale-100 opacity-100" : "scale-0 opacity-0",
          )}
          onPointerDown={onDragStart}
          draggable={false}
          title="Drag Window"
        >
          <Move className="w-full h-full p-1 text-black pointer-events-none" />
        </div>
      </InputGroup>

      {/* Hidden spacer to expand window when dropdowns are open */}
      <div
        className={cn(
          "pointer-events-none overflow-hidden transition-all duration-0",
          isPlusDropdownOpen && "h-[112px]",
          isToolsDropdownOpen && "h-[102px]",
          isModelDropdownOpen && "h-[155px]",
          !isPlusDropdownOpen &&
            !isToolsDropdownOpen &&
            !isModelDropdownOpen &&
            "h-0 delay-[50ms]",
        )}
      />
    </div>
  );
}

export default HUDInputBar;
