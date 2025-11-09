'use client';

import React, { createContext, useContext, useReducer, ReactNode, createRef, useRef } from 'react';
import { WindowsState } from './types';

/**
 * Initial state for windows
 */
const initialState: WindowsState = {
  isChatExpanded: false,
  isFeaturesExpanded: false,
  isChatHistoryExpanded: false,
};

/**
 * Action types for the windows reducer
 */
type WindowsAction =
  | { type: 'SET_MINIMIZED_CHAT'; }
  | { type: 'SET_EXPANDED_CHAT'; }
  | { type: 'SET_CHAT_HISTORY_EXPANDED'; }
  | { type: 'SET_CHAT_HISTORY_COLLAPSED'; }

/**
 * Windows reducer - handles all state updates
 */
function windowsReducer(
  state: WindowsState,
  action: WindowsAction
): WindowsState {
  switch (action.type) {
    case 'SET_MINIMIZED_CHAT':
      return {
        ...state,
        isChatExpanded: false,
      };

    case 'SET_EXPANDED_CHAT':
      return {
        ...state,
        isChatExpanded: true,
      };

    case 'SET_CHAT_HISTORY_EXPANDED':
      return {
        ...state,
        isChatHistoryExpanded: true,
      };

    case 'SET_CHAT_HISTORY_COLLAPSED':
      return {
        ...state,
        isChatHistoryExpanded: false,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface WindowsContextType {
  state: WindowsState;
  dispatch: React.Dispatch<WindowsAction>;
}

/**
 * Windows Context
 */
const WindowsContext = createContext<WindowsContextType | undefined>(undefined);

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

/**
 * Hook to access conversation context
 * Must be used within a WindowsProvider
 */
export function useWindowsContext(): WindowsContextType {
  const context = useContext(WindowsContext);
  
  if (!context) {
    throw new Error('useWindowsContext must be used within a WindowsProvider');
  }
  
  return context;
}
