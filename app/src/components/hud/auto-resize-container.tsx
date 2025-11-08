'use client';

import React, { useEffect, useRef, ReactNode } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { HudDimensions } from '@/types/settings';

interface AutoResizeContainerProps {
  children: ReactNode;
  hudDimensions: HudDimensions | null;
  className?: string;
}

/**
 * AutoResizeContainer - A wrapper component that automatically resizes the Tauri window
 * based on its content size using ResizeObserver.
 */
export function AutoResizeContainer({ 
  children, 
  hudDimensions,
  className = '' 
}: AutoResizeContainerProps) {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const lastHeightRef = useRef<number | null>(null);

  useEffect(() => {
    if (!containerRef.current || !hudDimensions) {
      return;
    }

    const container = containerRef.current;

    const resizeWindow = async () => {
      if (!container || !hudDimensions) return;

      // Get the bounding rectangle for accurate sizing (Svelte approach)
      const rect = container.getBoundingClientRect();
      
      // Calculate the actual height in CSS pixels
      const contentHeight = Math.ceil(rect.height);
      
      // Skip if height hasn't changed (avoid unnecessary backend calls)
      if (contentHeight === lastHeightRef.current) {
        return;
      }
      
      lastHeightRef.current = contentHeight;

      try {
        // Call backend to resize the window
        await invoke('resize_hud', {
          width: hudDimensions.chat_width,
          height: contentHeight
        });
      } catch (error) {
        console.error('[AutoResizeContainer] Failed to resize window:', error);
      }
    };

    // Set up ResizeObserver to watch for content changes
    const observer = new ResizeObserver((entries) => {
      resizeWindow();
    });

    observer.observe(container);

    // Initial resize
    resizeWindow();

    // Cleanup
    return () => {
      observer.disconnect();
    };
  }, [hudDimensions]);

  return (
    <div ref={containerRef} className={className}>
      {children}
    </div>
  );
}

export default AutoResizeContainer;
