"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

interface SelectionBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

interface ScreenSelectionResult {
  bounds: SelectionBounds;
  text_content: string;
  raw_data: unknown[];
}

export default function ScreenSelectorPage() {
  const [isSelecting, setIsSelecting] = useState(false);
  const [selectionStart, setSelectionStart] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [selectionEnd, setSelectionEnd] = useState<{
    x: number;
    y: number;
  } | null>(null);
  const [screenDimensions, setScreenDimensions] = useState<{
    width: number;
    height: number;
  } | null>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  // Initialize screen dimensions and add transparent background class
  useEffect(() => {
    // Add transparent background class
    document.documentElement.classList.add("screen-selector-transparent");
    document.body.classList.add("screen-selector-transparent");

    // Get screen dimensions
    const loadScreenDimensions = async () => {
      try {
        const [width, height] = await invoke<[number, number]>(
          "get_screen_dimensions",
        );
        setScreenDimensions({ width, height });
      } catch (error) {
        console.error("Failed to get screen dimensions:", error);
      }
    };

    void loadScreenDimensions();

    // Cleanup function
    return () => {
      if (typeof document !== "undefined") {
        document.documentElement.classList.remove(
          "screen-selector-transparent",
        );
        document.body.classList.remove("screen-selector-transparent");
      }
    };
  }, []);

  const cancelSelector = useCallback(async () => {
    try {
      await invoke("cancel_screen_selection");
      await invoke("close_screen_selector");
    } catch (error) {
      console.error("Failed to cancel screen selector:", error);
    }
  }, []);

  const closeSelector = useCallback(async () => {
    try {
      await invoke("close_screen_selector");
    } catch (error) {
      console.error("Failed to close screen selector:", error);
    }
  }, []);

  // Handle escape key to cancel selection
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        void cancelSelector();
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [cancelSelector]);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    const rect = overlayRef.current?.getBoundingClientRect();
    if (!rect) return;

    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    setIsSelecting(true);
    setSelectionStart({ x, y });
    setSelectionEnd({ x, y });
  }, []);

  const handleMouseMove = useCallback(
    (e: React.MouseEvent) => {
      if (!isSelecting || !selectionStart) return;

      const rect = overlayRef.current?.getBoundingClientRect();
      if (!rect) return;

      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      setSelectionEnd({ x, y });
    },
    [isSelecting, selectionStart],
  );

  const handleMouseUp = useCallback(async () => {
    if (!isSelecting || !selectionStart || !selectionEnd) return;

    setIsSelecting(false);

    // Calculate selection bounds
    const bounds: SelectionBounds = {
      x: Math.min(selectionStart.x, selectionEnd.x),
      y: Math.min(selectionStart.y, selectionEnd.y),
      width: Math.abs(selectionEnd.x - selectionStart.x),
      height: Math.abs(selectionEnd.y - selectionStart.y),
    };

    // Only process if selection has meaningful size
    if (bounds.width > 10 && bounds.height > 10) {
      try {
        await invoke<ScreenSelectionResult>("process_screen_selection", {
          bounds,
        });

        await closeSelector();
      } catch (error) {
        console.error("Failed to process screen selection:", error);
      } finally {
        // Reset selection
        setSelectionStart(null);
        setSelectionEnd(null);
      }
    } else {
      // Reset selection if too small
      setSelectionStart(null);
      setSelectionEnd(null);
    }
  }, [isSelecting, selectionStart, selectionEnd, closeSelector]);

  // Calculate selection rectangle for display
  const selectionRect =
    selectionStart && selectionEnd
      ? {
          left: Math.min(selectionStart.x, selectionEnd.x),
          top: Math.min(selectionStart.y, selectionEnd.y),
          width: Math.abs(selectionEnd.x - selectionStart.x),
          height: Math.abs(selectionEnd.y - selectionStart.y),
        }
      : null;

  return (
    <div
      ref={overlayRef}
      className="w-full h-full bg-black/10 cursor-crosshair select-none overflow-hidden"
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={() => {
        void handleMouseUp();
      }}
      style={{
        width: screenDimensions?.width ?? "100vw",
        height: screenDimensions?.height ?? "100vh",
      }}
    >
      {/* Semi-transparent overlay */}
      <div className="absolute inset-0 bg-black/20 pointer-events-none" />

      {/* Instructions */}
      <div className="absolute top-8 left-1/2 transform -translate-x-1/2 z-10 pointer-events-none">
        <div className="bg-white/90 backdrop-blur-sm rounded-lg px-4 py-2 shadow-lg">
          <p className="text-sm font-medium text-gray-800">
            Click and drag to select an area of the screen
          </p>
          <p className="text-xs text-gray-600 mt-1">Press ESC to cancel</p>
        </div>
      </div>

      {/* Selection rectangle */}
      {selectionRect && (
        <>
          {/* Clear area inside selection */}
          <div
            className="absolute bg-transparent border-2 border-blue-500 border-dashed pointer-events-none z-20"
            style={{
              left: selectionRect.left,
              top: selectionRect.top,
              width: selectionRect.width,
              height: selectionRect.height,
            }}
          />

          {/* Selection info */}
          <div
            className="absolute bg-blue-500 text-white text-xs px-2 py-1 rounded pointer-events-none z-30"
            style={{
              left: selectionRect.left,
              top: Math.max(0, selectionRect.top - 24),
            }}
          >
            {Math.round(selectionRect.width)} x{" "}
            {Math.round(selectionRect.height)}
          </div>
        </>
      )}

      {/* Corner handles when selection is active */}
      {selectionRect && !isSelecting && (
        <>
          {/* Top-left handle */}
          <div
            className="absolute w-2 h-2 bg-blue-500 border border-white pointer-events-none z-30"
            style={{
              left: selectionRect.left - 1,
              top: selectionRect.top - 1,
            }}
          />
          {/* Top-right handle */}
          <div
            className="absolute w-2 h-2 bg-blue-500 border border-white pointer-events-none z-30"
            style={{
              left: selectionRect.left + selectionRect.width - 1,
              top: selectionRect.top - 1,
            }}
          />
          {/* Bottom-left handle */}
          <div
            className="absolute w-2 h-2 bg-blue-500 border border-white pointer-events-none z-30"
            style={{
              left: selectionRect.left - 1,
              top: selectionRect.top + selectionRect.height - 1,
            }}
          />
          {/* Bottom-right handle */}
          <div
            className="absolute w-2 h-2 bg-blue-500 border border-white pointer-events-none z-30"
            style={{
              left: selectionRect.left + selectionRect.width - 1,
              top: selectionRect.top + selectionRect.height - 1,
            }}
          />
        </>
      )}
    </div>
  );
}
