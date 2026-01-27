"use client";

import type React from "react";
import { type ReactNode, createRef, useReducer, useRef } from "react";
import { type WindowsAction, WindowsContext } from "./WindowsContext";
import type { WindowsState } from "./types";

/**
 * Initial state for windows
 */
const initialState: WindowsState = {
  isChatExpanded: false,
  isFeaturesExpanded: false,
  isChatHistoryExpanded: false,
};

/**
 * Windows reducer - handles all state updates
 */
function windowsReducer(
  state: WindowsState,
  action: WindowsAction,
): WindowsState {
  switch (action.type) {
    case "SET_MINIMIZED_CHAT":
      return {
        ...state,
        isChatExpanded: false,
      };

    case "SET_EXPANDED_CHAT":
      return {
        ...state,
        isChatExpanded: true,
      };

    case "SET_CHAT_HISTORY_EXPANDED":
      return {
        ...state,
        isChatHistoryExpanded: true,
      };

    case "SET_CHAT_HISTORY_COLLAPSED":
      return {
        ...state,
        isChatHistoryExpanded: false,
      };

    default:
      return state;
  }
}

/**
 * Windows Provider Props
 */
interface WindowsProviderProps {
  children: ReactNode;
}

/**
 * Windows Provider Component
 * Wraps the application to provide shared windows state
 */
export function WindowsProvider({ children }: WindowsProviderProps) {
  const [state, dispatch] = useReducer(windowsReducer, initialState);

  return (
    <WindowsContext.Provider value={{ state, dispatch }}>
      {children}
    </WindowsContext.Provider>
  );
}
