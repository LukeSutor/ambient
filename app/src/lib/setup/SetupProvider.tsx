"use client";

import type React from "react";
import {
  type ReactNode,
  createContext,
  useContext,
  useReducer,
} from "react";

/**
 * Setup state
 */
interface SetupState {
  setupMessage: string;
  n_models: number;
  total_content_length: number;
  downloaded_bytes: number[]; // Stores the downloaded bytes for each model
  downloading_id: number | null;
}

/**
 * Initial state
 */
const initialState: SetupState = {
  setupMessage: "Initializing download...",
  n_models: 0,
  total_content_length: 0,
  downloaded_bytes: [],
  downloading_id: null,
};

/**
 * Action types
 */
type SetupAction =
  | { type: "SET_SETUP_MESSAGE"; payload: string }
  | { type: "SET_N_MODELS"; payload: number }
  | { type: "SET_TOTAL_CONTENT_LENGTH"; payload: number }
  | { type: "SET_DOWNLOADED_BYTES"; payload: { model_id: number; bytes: number } }
  | { type: "SET_DOWNLOADING_ID"; payload: number | null };

/**
 * Setup reducer
 */
function setupReducer(
  state: SetupState,
  action: SetupAction,
): SetupState {
  switch (action.type) {
    case "SET_SETUP_MESSAGE":
      return {
        ...state,
        setupMessage: action.payload,
      };

    case "SET_N_MODELS":
      return {
        ...state,
        n_models: action.payload,
      };

    case "SET_TOTAL_CONTENT_LENGTH":
      return {
        ...state,
        total_content_length: action.payload,
      };

    case "SET_DOWNLOADED_BYTES":
      return {
        ...state,
        downloaded_bytes: [
          ...state.downloaded_bytes.slice(0, action.payload.model_id),
          action.payload.bytes,
          ...state.downloaded_bytes.slice(action.payload.model_id + 1),
        ]
      };

    case "SET_DOWNLOADING_ID":
      return {
        ...state,
        downloading_id: action.payload,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface SetupContextType {
  state: SetupState;
  dispatch: React.Dispatch<SetupAction>;
}

/**
 * Setup Context
 */
const SetupContext = createContext<SetupContextType | undefined>(
  undefined,
);

/**
 * Setup Provider Props
 */
interface SetupProviderProps {
  children: ReactNode;
}

/**
 * Setup Provider Component
 * Provides shared setup state across the application
 */
export function SetupProvider({ children }: SetupProviderProps) {
  const [state, dispatch] = useReducer(setupReducer, initialState);

  return (
    <SetupContext.Provider value={{ state, dispatch }}>
      {children}
    </SetupContext.Provider>
  );
}

/**
 * Hook to access setup context
 * Must be used within a SetupProvider
 */
export function useSetupContext(): SetupContextType {
  const context = useContext(SetupContext);

  if (!context) {
    throw new Error(
      "useSetupContext must be used within a SetupProvider",
    );
  }

  return context;
}
