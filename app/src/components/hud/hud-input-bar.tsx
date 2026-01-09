'use client';

import React, { useRef, useState } from 'react';
import TextareaAutosize from "react-textarea-autosize";
import { Button } from "@/components/ui/button";
import {
  InputGroup,
  InputGroupAddon,
  InputGroupButton,
} from "@/components/ui/input-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuSeparator,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { ScrollArea, ScrollBar } from "@/components/ui/scroll-area"
import { MessageSquarePlus, Move, Plus, SquareDashedMousePointer, X, History, ArrowUpIcon, Settings2, ChevronDown, MousePointerClick, Wrench } from 'lucide-react';
import OcrCaptures from './ocr-captures';
import { OcrResponseEvent } from '@/types/events';
import { HudDimensions, ModelSelection } from '@/types/settings';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import { useWindows } from '@/lib/windows/useWindows';
import { useSettings } from '@/lib/settings';

interface HUDInputBarProps {
  hudDimensions: HudDimensions | null;
  inputValue: string;
  setInputValue: (v: string) => void;
  handleSubmit: () => Promise<void>;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement | HTMLTextAreaElement>) => void;
  dispatchOCRCapture: () => void;
  deleteOCRResult: (index: number) => void;
  onNewChat: () => void;
  onDragStart: () => void;
  onMouseLeave: (e: React.MouseEvent) => void;
  isDraggingWindow: boolean;
  isHoveringGroup: boolean;
  setIsHoveringGroup: (b: boolean) => void;
  toggleComputerUse: () => void;
  ocrLoading: boolean;
  ocrResults: OcrResponseEvent[];
  isStreaming: boolean;
  conversationType: string;
}

export function HUDInputBar({
  hudDimensions,
  inputValue,
  setInputValue,
  handleSubmit,
  onKeyDown,
  dispatchOCRCapture,
  deleteOCRResult,
  onNewChat,
  onDragStart,
  onMouseLeave,
  isDraggingWindow,
  isHoveringGroup,
  setIsHoveringGroup,
  toggleComputerUse,
  ocrLoading,
  ocrResults,
  isStreaming,
  conversationType,
}: HUDInputBarProps) {
  // Ref for load animation
  const inputRef = useRef<HTMLDivElement | null>(null);
  // Dimensions ref to check for changes
  const dimensionsRef = useRef<HudDimensions | null>(null);
  
  // Track dropdown open states
  const [isPlusDropdownOpen, setIsPlusDropdownOpen] = useState(false);
  const [isToolsDropdownOpen, setIsToolsDropdownOpen] = useState(false);
  const [isModelDropdownOpen, setIsModelDropdownOpen] = useState(false);
  
  // Settings hook for model selection
  const { settings, setModelSelection } = useSettings();
  const modelSelection = settings?.model_selection ?? 'Local';
  
  // Window Manager
  const {
    toggleChatHistory,
    closeHUD,
    openSecondary,
  } = useWindows();

  // Handle model selection change
  async function handleModelSelectionChange(value: string) {
    const newModel = value as ModelSelection;
    
    try {
      await setModelSelection(newModel);
    } catch (error) {
      console.error("Failed to save model selection setting:", error);
    }
  }

  // Animate input bar appearing
  useGSAP(() => {
    // Only animate if dimensions actually changed (deep comparison)
    if (dimensionsRef.current && hudDimensions && 
        JSON.stringify(dimensionsRef.current) === JSON.stringify(hudDimensions)) {
      return;
    }
    
    dimensionsRef.current = hudDimensions;
    
    if (hudDimensions && inputRef.current) {
      gsap.fromTo(
        inputRef.current,
        { scale: 0, opacity: 0, transformOrigin: 'center center' },
        { scale: 1, opacity: 1, duration: 0.25, ease: 'back.out(0.8)', delay: 0.1 }
      );
    }
  }, [hudDimensions]);


  return (
    <div
      className='flex flex-col justify-start items-center relative p-2'
      id="input-container"
      onMouseEnter={() => setIsHoveringGroup(true)}
      onMouseLeave={onMouseLeave}
      ref={inputRef}
      style={{
        minHeight: hudDimensions ? `${hudDimensions.input_bar_height}px` : '60px',
        width: hudDimensions ? `${hudDimensions.chat_width}px` : '500px',
        opacity: hudDimensions ? 1 : 0,
        transform: hudDimensions ? 'scale(1)' : 'scale(0)'
      }}
    >
      <InputGroup className="bg-white/60 border border-black/20 transition-all">
        <TextareaAutosize
          data-slot="input-group-control"
          maxRows={4}
          minRows={2}
          value={inputValue}
          onChange={(e) => setInputValue(e.target.value)}
          onKeyDown={onKeyDown}
          className="flex field-sizing-content hud-scroll min-h-16 w-full resize-none rounded-md bg-transparent px-3 py-2.5 text-base transition-[color,box-shadow] outline-none md:text-sm"
          placeholder="Ask anything"
          autoComplete="off"
          autoFocus
        />
        <InputGroupAddon align="block-end" className="-mb-2">
          {/* Plus dropdown menu */}
          <DropdownMenu onOpenChange={setIsPlusDropdownOpen}>
            <DropdownMenuTrigger asChild>
              <InputGroupButton
                variant="outline"
                className="rounded-full bg-white/60 hover:bg-white/80"
                size="icon-xs"
                disabled={ocrLoading || isStreaming}
              >
                <Plus />
              </InputGroupButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              side="bottom"
              align="start"
              avoidCollisions={false}
              sideOffset={12}
              alignOffset={-12}
              className="bg-white/60"
            >
              <DropdownMenuGroup>
                <DropdownMenuItem className="hover:bg-white/60" onClick={() => { onNewChat(); }}>
                  <MessageSquarePlus className="!w-4 !h-4 text-black shrink-0 mr-2" />
                  <span className="text-black text-sm whitespace-nowrap">New Chat</span>
                </DropdownMenuItem>
                <DropdownMenuItem className="hover:bg-white/60" onClick={() => { toggleChatHistory(); }}>
                  <History className="!w-4 !h-4 text-black shrink-0 mr-2" />
                  <span className="text-black text-sm whitespace-nowrap">Previous Chats</span>
                </DropdownMenuItem>
              </DropdownMenuGroup>
              <DropdownMenuSeparator />
              <DropdownMenuGroup>
                <DropdownMenuItem className="hover:bg-white/60" onClick={() => { openSecondary(); }}>
                  <Settings2 className="!w-4 !h-4 text-black shrink-0 mr-2" />
                  <span className="text-black text-sm whitespace-nowrap">Dashboard</span>
                </DropdownMenuItem>
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* Tools dropdown */}
          <DropdownMenu onOpenChange={setIsToolsDropdownOpen}>
            <DropdownMenuTrigger asChild>
              <InputGroupButton
                className="ml-2"
                variant="ghost"
                disabled={ocrLoading || isStreaming}
              >
                <Wrench className="mr-1" />
                Tools
              </InputGroupButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              side="bottom"
              align="start"
              avoidCollisions={false}
              sideOffset={12}
              alignOffset={-12}
              className="bg-white/60"
            >
              <DropdownMenuGroup>
                <DropdownMenuItem className="hover:bg-white/60" onClick={() => { dispatchOCRCapture(); }}>
                  <SquareDashedMousePointer className="!w-4 !h-4 text-black shrink-0 mr-2" />
                  <span className="text-black text-sm whitespace-nowrap">Capture Area</span>
                </DropdownMenuItem>
                <DropdownMenuItem className="hover:bg-white/60" onClick={() => { toggleComputerUse(); }}>
                  <MousePointerClick className="!w-4 !h-4 text-black shrink-0 mr-2" />
                  <span className="text-black text-sm whitespace-nowrap">Computer Use</span>
                </DropdownMenuItem>
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
          {/* Computer Use Icon */}

          {/* Horizonal scrollable div with computer use icon and ocr captures */}
          <ScrollArea className="min-w-0">
            <div className="flex w-max space-x-2 py-1">
              <div
                className={`flex items-center justify-center bg-yellow-500/30 rounded-xl shrink-0 overflow-hidden whitespace-nowrap transition-all duration-150
                  ${conversationType === "computer_use" ? "px-2 py-1" : "p-0 w-0"}`}
              >
                <MousePointerClick className="!h-4 !w-4 text-black" />
                <p className="mx-1 text-black">Computer Use</p>
                <Button
                  variant="ghost"
                  className="!h-4 !w-4 text-black shrink-0 hover:bg-transparent"
                  size="icon"
                  onClick={() => toggleComputerUse()}
                >
                  <X className="!h-3 !w-3 text-black shrink-0" />
                </Button>
              </div>
              <OcrCaptures hud-scrolls captures={ocrResults} ocrLoading={ocrLoading} onRemove={deleteOCRResult} />
              {/* Make sure the height stays constant */}
              <div className="h-6" />
            </div>
            <ScrollBar orientation="horizontal" className="[&_[data-slot='scroll-area-thumb']]:bg-black/25 [&_[data-slot='scroll-area-thumb']]:hover:bg-black/30" />
          </ScrollArea>
          <DropdownMenu onOpenChange={setIsModelDropdownOpen}>
            <DropdownMenuTrigger asChild>
              <InputGroupButton className="ml-auto" variant="ghost" disabled={ocrLoading || isStreaming}>
                {modelSelection === "Local" && "Local"}
                {modelSelection === "GptOss" && "GPT OSS"}
                {modelSelection === "Gpt5" && "GPT-5"}
                <ChevronDown />
              </InputGroupButton>
            </DropdownMenuTrigger>
            <DropdownMenuContent
              side="bottom"
              align="start"
              avoidCollisions={false}
              sideOffset={12}
              alignOffset={-115}
              className="w-full bg-white/60"
            >
              <DropdownMenuGroup>
                <DropdownMenuItem 
                  onClick={() => handleModelSelectionChange('Local')}
                  className="py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60"
                >
                  <span className="font-medium text-sm">Local</span>
                  <span className="text-xs text-muted-foreground">
                    Ultimate privacy. Runs on your device.
                  </span>
                </DropdownMenuItem>
                <DropdownMenuItem 
                  onClick={() => handleModelSelectionChange('GptOss')}
                  className="py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60"
                >
                  <span className="font-medium text-sm">GPT OSS</span>
                  <span className="text-xs text-muted-foreground">
                    More powerful open-source model.
                  </span>
                </DropdownMenuItem>
                <DropdownMenuItem
                  onClick={() => handleModelSelectionChange('Gpt5')}
                  className="py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60"
                >
                  <span className="font-medium text-sm">GPT-5</span>
                  <span className="text-xs text-muted-foreground">
                    The latest and most advanced model.
                  </span>
                </DropdownMenuItem>
              </DropdownMenuGroup>
            </DropdownMenuContent>
          </DropdownMenu>
          <InputGroupButton
            variant="default"
            className="rounded-full bg-black/80 hover:bg-black"
            size="icon-xs"
            type="submit"
            onClick={handleSubmit}
            disabled={ocrLoading || isStreaming}
          >
            <ArrowUpIcon />
            <span className="sr-only">Send</span>
          </InputGroupButton>
        </InputGroupAddon>
          {/* Close icon */}
          <button
            className={(isDraggingWindow || isHoveringGroup ? 'scale-100 opacity-100' : 'scale-0 opacity-0') +
              ' absolute -top-1.5 -right-1.5 w-6 h-6 rounded-full bg-white/60 hover:bg-white/80 border border-black/20 transition-all duration-100 select-none'}
            onClick={closeHUD}
            title="Close Window"
          >
            <X className="w-full h-full p-1 text-black pointer-events-none" />
          </button>
          {/* Move handle */}

          <div
            data-tauri-drag-region
            id="drag-area"
            className={(isDraggingWindow || isHoveringGroup ? 'scale-100 opacity-100' : 'scale-0 opacity-0') +
              ' hover:cursor-grab select-none absolute -bottom-1.5 -right-1.5 w-6 h-6 bg-white/60 hover:bg-white/80 border border-black/20 rounded-full transition-all duration-100'}
            onPointerDown={onDragStart}
            draggable={false}
            title="Drag Window"
          >
            <Move className="w-full h-full p-1 text-black pointer-events-none" />
          </div>
        </InputGroup>

      {/* Hidden spacer to expand window when dropdowns are open */}
      <div 
        className={`pointer-events-none overflow-hidden ${
          isPlusDropdownOpen
            ? 'h-[110px] transition-none'
            : isToolsDropdownOpen
            ? 'h-[70px] transition-none'
            : isModelDropdownOpen
            ? 'h-[155px] transition-none'
            : 'h-0 transition-all duration-0 delay-[50ms]'
        }`}
      />
    </div>
  );
}

export default HUDInputBar;
