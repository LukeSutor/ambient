"use client";

import type {
  DownloadFinishedEvent,
  DownloadInformationEvent,
  DownloadProgressEvent,
  DownloadStartedEvent,
} from "@/types/events";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { formatBytes } from "../utils";
import { useSetupContext } from "./SetupProvider";

/**
 * Main setup hook - provides all setup functionality
 * @returns Setup state and operations
 */
export function useSetup() {
  const { state, dispatch } = useSetupContext();
  const cleanupRef = useRef<(() => void) | null>(null);

  // Use a ref to keep track of the latest state in async callbacks without re-running effects
  const stateRef = useRef(state);
  useEffect(() => {
    stateRef.current = state;
  }, [state]);

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
          listen<DownloadInformationEvent>("download_information", (event) => {
            console.log({ event });
            dispatch({
              type: "SET_N_MODELS",
              payload: Number(event.payload.n_items),
            });
            dispatch({
              type: "SET_TOTAL_CONTENT_LENGTH",
              payload: Number(event.payload.content_length),
            });
          }),
          listen<DownloadStartedEvent>("download_started", (event) => {
            console.log({ event });
            dispatch({ type: "SET_IS_DOWNLOADING", payload: true });
            dispatch({
              type: "SET_DOWNLOADING_ID",
              payload: Number(event.payload.id),
            });
          }),
          listen<DownloadProgressEvent>("download_progress", (event) => {
            dispatch({
              type: "SET_DOWNLOADED_BYTES",
              payload: {
                model_id: Number(event.payload.id),
                bytes: Number(event.payload.total_progress),
              },
            });
          }),
          listen<DownloadFinishedEvent>("download_finished", (event) => {
            console.log({ event });
            dispatch({ type: "SET_DOWNLOADING_ID", payload: null });
            // If all downloads are complete, update state
            if (Number(event.payload.id) === stateRef.current.numModels) {
              dispatch({ type: "SET_IS_DOWNLOADING", payload: false });
            }
          }),
        ];

        const results = await Promise.all(listenerPromises);
        unlisteners.push(...results);

        console.log("[useSetup] Event listeners initialized");
      } catch (error) {
        console.error("[useSetup] Failed to setup event listener:", error);
      }
    };

    // Fetch total bytes needed and number of items for download
    const fetchDownloadInfo = async () => {
      try {
        const info = await invoke<DownloadInformationEvent>(
          "get_setup_download_info",
        );
        console.log({ info });
        dispatch({ type: "SET_N_MODELS", payload: Number(info.n_items) });
        dispatch({
          type: "SET_TOTAL_CONTENT_LENGTH",
          payload: Number(info.content_length),
        });
      } catch (error) {
        console.error(
          "[useSetup] Failed to fetch download information:",
          error,
        );
      }
    };

    void fetchDownloadInfo();
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
    dispatch({ type: "SET_IS_DOWNLOADING", payload: true });
    try {
      const result = await invoke<string>("setup");
      console.log("[useSetup] Setup command finished:", result);
    } catch (error) {
      console.error("[useSetup] Setup command failed:", error);
      dispatch({ type: "SET_IS_DOWNLOADING", payload: false });
      throw error;
    } finally {
      dispatch({ type: "SET_IS_DOWNLOADING", payload: false });
    }
  }, [dispatch]);

  const totalDownloadedBytes = useMemo(() => {
    return state.downloadedBytes.reduce((a, b) => a + b, 0);
  }, [state.downloadedBytes]);

  const formattedDownloadedBytes = useMemo(() => {
    return formatBytes(totalDownloadedBytes);
  }, [totalDownloadedBytes]);

  const formattedTotalContentLength = useMemo(() => {
    return formatBytes(state.totalContentLength);
  }, [state.totalContentLength]);

  // ============================================================
  // Return API
  // ============================================================

  return {
    // State
    ...state,
    totalDownloadedBytes,
    formattedDownloadedBytes,
    formattedTotalContentLength,

    // Operations
    startSetup,
  };
}
