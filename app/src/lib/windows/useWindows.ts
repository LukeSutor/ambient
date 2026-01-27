"use client";

import { invoke } from "@tauri-apps/api/core";
import { useCallback } from "react";
import { useWindowsContext } from "./WindowsContext";

export function useWindows() {
  const { state, dispatch } = useWindowsContext();

  // ============================================================
  // Operations
  // ============================================================
  const setChatMinimized = useCallback(
    (delay?: number) => {
      if (delay) {
        setTimeout(() => {
          dispatch({ type: "SET_MINIMIZED_CHAT" });
        }, delay);
      } else {
        dispatch({ type: "SET_MINIMIZED_CHAT" });
      }
    },
    [dispatch],
  );

  const setChatExpanded = useCallback(() => {
    dispatch({ type: "SET_EXPANDED_CHAT" });
  }, [dispatch]);

  const toggleChatHistory = useCallback(
    async (nextState?: boolean) => {
      const willExpand = nextState ?? !state.isChatHistoryExpanded;

      if (willExpand) {
        dispatch({ type: "SET_CHAT_HISTORY_EXPANDED" });
      } else {
        dispatch({ type: "SET_CHAT_HISTORY_COLLAPSED" });
      }
    },
    [state.isChatHistoryExpanded, dispatch],
  );

  const closeHUD = useCallback(async () => {
    try {
      await invoke("close_main_window");
    } catch (error) {
      console.error("Failed to close window:", error);
    }
  }, []);

  const openSecondary = useCallback(async (destination?: string) => {
    try {
      await invoke("open_secondary_window", {
        destination: destination || null,
      });
    } catch (error) {
      console.error("Failed to open secondary window:", error);
    }
  }, []);

  // ============================================================
  // Return API
  // ============================================================
  return {
    ...state,
    // Operations
    setChatMinimized,
    setChatExpanded,
    toggleChatHistory,
    closeHUD,
    openSecondary,
  };
}
