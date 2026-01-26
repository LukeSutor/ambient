"use client";

import { useSettings } from "@/lib/settings/useSettings";
import type { HudDimensions } from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import React, { useEffect, useRef, useState, type ReactNode } from "react";

interface AutoResizeContainerProps {
  children: ReactNode;
  widthType: string;
  className?: string;
}

/**
 * AutoResizeContainer - A wrapper component that automatically resizes the Tauri window
 * based on its content size using ResizeObserver.
 */
export function AutoResizeContainer({
  children,
  widthType,
  className = "",
}: AutoResizeContainerProps) {
  const { getHudDimensions } = useSettings();
  const [hudDimensions, setHudDimensions] = useState<HudDimensions | null>(
    null,
  );

  const containerRef = useRef<HTMLDivElement | null>(null);
  const lastHeightRef = useRef<number | null>(null);

  useEffect(() => {
    void (async () => {
      const dimensions = await getHudDimensions();
      setHudDimensions(dimensions);
    })();
  }, [getHudDimensions]);

  useEffect(() => {
    if (!containerRef.current || !hudDimensions) {
      return;
    }

    const container = containerRef.current;

    const resizeWindow = async () => {
      // Get container height
      const rect = container.getBoundingClientRect();
      const contentHeight = Math.ceil(rect.height);

      // Skip if height hasn't changed
      if (contentHeight === lastHeightRef.current) {
        return;
      }
      lastHeightRef.current = contentHeight;

      // Get the correct width
      let width = hudDimensions.chat_width;
      if (widthType === "login") {
        width = hudDimensions.login_width;
      }

      try {
        // Call backend to resize the window
        await invoke("resize_main_window", {
          width: width,
          height: contentHeight,
        });
      } catch (error) {
        console.error("[AutoResizeContainer] Failed to resize window:", error);
      }
    };

    // Set up ResizeObserver to watch for content changes
    const observer = new ResizeObserver((_entries) => {
      void resizeWindow();
    });

    observer.observe(container);

    // Initial resize
    void resizeWindow();

    // Cleanup
    return () => {
      observer.disconnect();
    };
  }, [hudDimensions, widthType]);

  return (
    <div ref={containerRef} className={className}>
      {children}
    </div>
  );
}

export default AutoResizeContainer;
