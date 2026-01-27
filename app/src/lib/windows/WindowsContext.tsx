"use client";

import type React from "react";
import { createContext, useContext } from "react";
import type { WindowsState } from "./types";

/**
 * Action types for the windows reducer
 */
export type WindowsAction =
  | { type: "SET_MINIMIZED_CHAT" }
  | { type: "SET_EXPANDED_CHAT" }
  | { type: "SET_CHAT_HISTORY_EXPANDED" }
  | { type: "SET_CHAT_HISTORY_COLLAPSED" };

/**
 * Context type
 */
export interface WindowsContextType {
  state: WindowsState;
  dispatch: React.Dispatch<WindowsAction>;
}

/**
 * Windows Context
 */
export const WindowsContext = createContext<WindowsContextType | undefined>(
  undefined,
);

/**
 * Hook to access conversation context
 * Must be used within a WindowsProvider
 */
export function useWindowsContext(): WindowsContextType {
  const context = useContext(WindowsContext);

  if (!context) {
    throw new Error("useWindowsContext must be used within a WindowsProvider");
  }

  return context;
}
