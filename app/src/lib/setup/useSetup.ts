"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo } from "react";
import { formatBytes } from "../utils";
import { useSetupContext } from "./SetupProvider";

/**
 * Main setup hook - provides all setup functionality
 * @returns Setup state and operations
 */
export function useSetup() {
  const { state, dispatch } = useSetupContext();

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
