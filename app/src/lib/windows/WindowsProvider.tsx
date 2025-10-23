'use client';

import React, { createContext, useContext, useReducer, ReactNode, useRef, RefObject } from 'react';
import { WindowsState } from './types';

/**
 * Initial state for windows
 */
const initialState: WindowsState = {
  isLogin: true,
  isChatExpanded: false,
  isFeaturesExpanded: false,
  settingsDestination: '',
  messagesContainerRef: React.createRef<null>(),
  featuresRef: React.createRef<null>(),
};

/**
 * Action types for the windows reducer
 */
type WindowsAction =
  | { type: 'SET_LOGIN'; }
  | { type: 'SET_MINIMIZED_CHAT'; }
  | { type: 'SET_EXPANDED_CHAT'; }
  | { type: 'SET_FEATURES_EXPANDED'; }
  | { type: 'SET_FEATURES_COLLAPSED'; }
  | { type: 'OPEN_SETTINGS'; payload?: string; };

/**
 * Windows reducer - handles all state updates
 */
function windowsReducer(
  state: WindowsState,
  action: WindowsAction
): WindowsState {
  switch (action.type) {
    case 'SET_LOGIN':
      return {
        ...state,
        isLogin: true,
      };

    case 'SET_MINIMIZED_CHAT':
      return {
        ...state,
        isLogin: false,
        isChatExpanded: false,
      };

    case 'SET_EXPANDED_CHAT':
      return {
        ...state,
        isLogin: false,
        isChatExpanded: true,
      };

    case 'SET_FEATURES_EXPANDED':
      return {
        ...state,
        isLogin: false,
        isFeaturesExpanded: true,
      };

    case 'SET_FEATURES_COLLAPSED':
      return {
        ...state,
        isLogin: false,
        isFeaturesExpanded: false,
      };

    case 'OPEN_SETTINGS':
      return {
        ...state,
        settingsDestination: action.payload || '',
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
  
  // Store refs at the provider level so they persist across renders
  const messagesContainerRef = useRef<HTMLDivElement | null>(null);
  const featuresRef = useRef<HTMLDivElement | null>(null);

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
