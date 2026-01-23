"use client";

import { DownloadInformationEvent, DownloadStartedEvent, DownloadProgressEvent, DownloadFinishedEvent } from "@/types/events";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef } from "react";
import { useSetupContext } from "./SetupProvider";

/**
 * Main setup hook - provides all setup functionality
 * @returns Setup state and operations
 */
export function useSetup() {
  const { state, dispatch } = useSetupContext();
  const cleanupRef = useRef<(() => void) | null>(null);

  // ============================================================
  // Event Listener Setup
  // ============================================================
  useEffect(() => {
    let isMounted = true;
    const unlisteners: UnlistenFn[] = [];

    const setupEvents = async () => {
      // Clean up previous listeners
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }

      if (!isMounted) return;

      try {
        console.log("[useSetup] Setting up event listeners...");

        // Listen for download events
        const listenerPromises = [
          listen<DownloadInformationEvent>("download-information", (event) => {
            dispatch({ type: "SET_N_MODELS", payload: Number(event.payload.n_items) });
            dispatch({ type: "SET_TOTAL_CONTENT_LENGTH", payload: Number(event.payload.content_length) });
          }),
          listen<DownloadStartedEvent>("download-started", (event) => {
            dispatch({ type: "SET_DOWNLOADING_ID", payload: Number(event.payload.id) });
            dispatch({ type: "SET_SETUP_MESSAGE", payload: `Downloading Model ${event.payload.id} of ${state.n_models}` });
          }),
          listen<DownloadProgressEvent>("download-progress", (event) => {
            dispatch({ type: "SET_DOWNLOADED_BYTES", payload: { model_id: Number(event.payload.id), bytes: Number(event.payload.total_progress)} });
          }),
          listen<DownloadFinishedEvent>("download-finished", (event) => {
            dispatch({ type: "SET_DOWNLOADING_ID", payload: null });
            dispatch({ type: "SET_SETUP_MESSAGE", payload: `Model ${event.payload.id} download complete.` });
          }),
        ];

        const results = await Promise.all(listenerPromises);
        unlisteners.push(...results);

        console.log("[useSetup] Event listeners initialized");
      } catch (error) {
        console.error("[useSetup] Failed to setup event listener:", error);
      }
    };

    //TODO: Fetch total bytes needed for download on mount for display

    void setupEvents();

    // Store cleanup function
    cleanupRef.current = () => {
      for (const unlisten of unlisteners) {
        try {
          unlisten();
        } catch (error) {
          console.error("[useSetup] Error during cleanup:", error);
        }
      }
      console.log("[useSetup] Event listeners cleaned up");
    };

    // Cleanup on unmount
    return () => {
      isMounted = false;

      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, [dispatch]);

  // ============================================================
  // Operations
  // ============================================================

  const startSetup = useCallback(async () => {
    try {
      const result = await invoke<string>("setup");
      console.log("[useSetup] Setup command finished:", result);
    } catch (error) {
      console.error("[useSetup] Setup command failed:", error);
      throw error;
    }
  }, []);

  // ============================================================
  // Return API
  // ============================================================

  return {
    // State
    ...state,
    startSetup,
    };
}
