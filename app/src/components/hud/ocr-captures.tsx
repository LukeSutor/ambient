'use client';

import React from 'react';
import { Button } from '@/components/ui/button';
import { SquareDashed, X } from 'lucide-react';
import { OcrResponseEvent } from '@/types/events';

interface OcrCapturesProps {
  captures: OcrResponseEvent[];
  ocrLoading: boolean;
  onRemove: (index: number) => void;
}

export function OcrCaptures({ captures, ocrLoading, onRemove }: OcrCapturesProps) {
  return (
    <div className="flex items-center gap-1 overflow-hidden whitespace-nowrap shrink min-w-0">
      {captures.map((capture, index) => (
        <div
          key={index}
          className="flex items-center justify-center bg-blue-500/30 rounded-xl px-2 py-1 shrink-0"
          title={capture.text.length > 15 ? capture.text.slice(0, 15) + '...' : capture.text}
        >
          <SquareDashed className="!h-4 !w-4 text-black" />
          <Button
            variant="ghost"
            className="!h-4 !w-4 text-black shrink-0 hover:bg-transparent"
            size="icon"
            onClick={() => onRemove(index)}
          >
            <X className="!h-3 !w-3 text-black shrink-0" />
          </Button>
        </div>
      ))}
      {/* Display a loading ocr item */}
      {ocrLoading && (
        <div className="flex items-center justify-center bg-blue-500/30 rounded-xl px-2 py-1 shrink-0">
          <SquareDashed className="!h-4 !w-4 text-black" />
          <Button
            variant="ghost"
            className="!h-4 !w-4 text-black shrink-0 hover:bg-transparent"
            size="icon"
            disabled
            >
            <X className="!h-3 !w-3 text-black shrink-0" />
          </Button>
        </div>
      )}
    </div>
  );
}

export default OcrCaptures;
