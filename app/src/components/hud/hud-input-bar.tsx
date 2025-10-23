'use client';

import React, { forwardRef, useRef } from 'react';
import Image from 'next/image';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { LoaderCircle, MessageSquarePlus, Move, Plus, SquareDashedMousePointer, X } from 'lucide-react';
import OcrCaptures from './ocr-captures';
import { OcrResponseEvent } from '@/types/events';
import { HudDimensions } from '@/types/settings';
import { useWindows } from '@/lib/windows/useWindows';

interface HUDInputBarProps {
  hudDimensions: HudDimensions | null;
  inputValue: string;
  setInputValue: (v: string) => void;
  onKeyDown: (e: React.KeyboardEvent<HTMLInputElement>) => void;
  onCaptureArea: () => void;
  onNewChat: () => void;
  onDragStart: () => void;
  onMouseLeave: (e: React.MouseEvent) => void;
  isDraggingWindow: boolean;
  isHoveringGroup: boolean;
  setIsHoveringGroup: (b: boolean) => void;
  ocrLoading: boolean;
  ocrResults: OcrResponseEvent[];
  removeOcrAt: (i: number) => void;
}

const logo = '/logo.png';

export const HUDInputBar = forwardRef<HTMLDivElement, HUDInputBarProps>(function HUDInputBar({
  hudDimensions,
  inputValue,
  setInputValue,
  onKeyDown,
  onCaptureArea,
  onNewChat,
  onDragStart,
  onMouseLeave,
  isDraggingWindow,
  isHoveringGroup,
  setIsHoveringGroup,
  ocrLoading,
  ocrResults,
  removeOcrAt,
}, ref) {
  // Window Manager
  const {
    isFeaturesExpanded,
    isChatExpanded,
    featuresRef: windowsFeaturesRef,
    toggleFeatures,
    closeHUD,
    openSettings,
  } = useWindows();
  
  const featuresDropdownRef = useRef<HTMLDivElement | null>(null);

  // Sync local ref with windows manager ref
  React.useEffect(() => {
    if (featuresDropdownRef.current) {
      windowsFeaturesRef.current = featuresDropdownRef.current;
    }
  }, [windowsFeaturesRef]);

  return (
    <div
      className='flex-shrink-0 flex flex-col justify-center items-center relative p-2'
      id="input-container"
      onMouseEnter={() => setIsHoveringGroup(true)}
      onMouseLeave={onMouseLeave}
      ref={ref}
      style={{
        height: hudDimensions ? `${hudDimensions.input_bar_height}px` : '60px',
        width: hudDimensions ? `${hudDimensions.chat_width}px` : '500px',
        opacity: hudDimensions ? 1 : 0,
        transform: hudDimensions ? 'scale(1)' : 'scale(0)'
      }}
    >
      <div
        className='flex items-center gap-3 rounded-lg bg-white/60 border border-black/20 transition-all focus-within:outline-none focus-within:ring-0 focus-within:border-black/20 flex-1 w-full'
      >
        <button onClick={() => openSettings()} title="Open Main Window" className="shrink-0">
          <Image
            src={logo}
            width={32}
            height={32}
            alt="Logo"
            className="w-7 h-7 ml-2 select-none pointer-events-none shrink-0"
            draggable={false}
            onDragStart={(e) => e.preventDefault()}
          />
        </button>

        <div className="flex-1 min-w-32">
          <Input
            type="text"
            value={inputValue}
            onChange={(e) => setInputValue(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder="Ask anything"
            className="bg-transparent rounded-none border-none shadow-none p-0 text-black placeholder:text-black/75 transition-all outline-none ring-0 focus:outline-none focus:ring-0 focus:ring-offset-0 focus-visible:outline-none focus-visible:ring-0 focus-visible:ring-offset-0 min-w-0 w-full"
            autoComplete="off"
            autoFocus
          />
        </div>

        <OcrCaptures captures={ocrResults} onRemove={removeOcrAt} />

        {/* Additional features expandable area */}
        <div className={`relative flex flex-row justify-end items-center w-auto min-w-8 h-8 rounded-full hover:bg-white/60 mr-5 transition-all ${isFeaturesExpanded ? 'bg-white/40' : ''} shrink-0`}>
          <div className={`absolute mb-1 right-0 bg-white/40 border border-black/20 rounded-lg p-2 flex flex-col gap-2 transition-all duration-250 ease-in-out overflow-hidden ${isFeaturesExpanded ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-2 pointer-events-none'} ${isChatExpanded ? 'bottom-full' : 'top-full'}`} ref={featuresDropdownRef}>
            <Button
              variant="ghost"
              className="flex items-center gap-2 h-8 px-3 rounded-md hover:bg-white/60 justify-start"
              onClick={() => { onCaptureArea(); toggleFeatures(); }}
              title="Capture Area"
            >
              <SquareDashedMousePointer className="!w-4 !h-4 text-black shrink-0" />
              <span className="text-black text-sm whitespace-nowrap">Capture Area</span>
            </Button>
            <Button
              variant="ghost"
              className="flex items-center gap-2 h-8 px-3 rounded-md hover:bg-white/60 justify-start"
              onClick={() => { onNewChat(); toggleFeatures(); }}
              title="New Chat"
            >
              <MessageSquarePlus className="!w-4 !h-4 text-black shrink-0" />
              <span className="text-black text-sm whitespace-nowrap">New Chat</span>
            </Button>
          </div>
          <Button
            variant="ghost"
            className="w-8 h-8 rounded-full"
            size="icon"
            disabled={ocrLoading}
            onClick={() => toggleFeatures()}
          >
            {ocrLoading ? <LoaderCircle className="!h-5 !w-5 animate-spin" /> : <Plus className={`!h-5 !w-5 text-black shrink-0 transition-transform duration-300 ${isFeaturesExpanded ? 'rotate-45' : 'rotate-0'}`} />}
          </Button>
        </div>
      </div>

      {/* Close icon */}
      <button
        className={(isDraggingWindow || isHoveringGroup ? 'scale-100 opacity-100' : 'scale-0 opacity-0') +
          ' absolute top-0.5 right-0.5 w-6 h-6 rounded-full bg-white/60 hover:bg-white/80 border border-black/20 transition-all duration-100 select-none'}
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
          ' hover:cursor-grab select-none absolute bottom-0.5 right-0.5 w-6 h-6 bg-white/60 hover:bg-white/80 border border-black/20 rounded-full transition-all duration-100'}
        onPointerDown={onDragStart}
        draggable={false}
        title="Drag Window"
      >
        <Move className="w-full h-full p-1 text-black pointer-events-none" />
      </div>
    </div>
  );
});

export default HUDInputBar;
