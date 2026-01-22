"use client";

import type {
  HudSizeOption,
  ModelSelection,
  UserSettings,
} from "@/types/settings";
import type React from "react";
import {
  type MutableRefObject,
  type ReactNode,
  createContext,
  useContext,
  useReducer,
} from "react";

/**
 * Settings state
 */
interface SettingsState {
  settings: UserSettings | null;
  isLoading: boolean;
  initializationRef: MutableRefObject<boolean>;
}

/**
 * Initial state
 */
const initialState: SettingsState = {
  settings: null,
  isLoading: true,
  initializationRef: { current: false },
};

/**
 * Action types
 */
type SettingsAction =
  | { type: "SET_SETTINGS"; payload: UserSettings }
  | { type: "UPDATE_HUD_SIZE"; payload: HudSizeOption }
  | { type: "UPDATE_MODEL_SELECTION"; payload: ModelSelection }
  | { type: "SET_LOADING"; payload: boolean }
  | { type: "INVALIDATE_CACHE" };

/**
 * Settings reducer
 */
function settingsReducer(
  state: SettingsState,
  action: SettingsAction,
): SettingsState {
  switch (action.type) {
    case "SET_SETTINGS":
      return {
        ...state,
        settings: action.payload,
        isLoading: false,
      };

    case "UPDATE_HUD_SIZE":
      if (!state.settings) return state;
      return {
        ...state,
        settings: {
          ...state.settings,
          hud_size: action.payload,
        },
      };

    case "UPDATE_MODEL_SELECTION":
      if (!state.settings) return state;
      return {
        ...state,
        settings: {
          ...state.settings,
          model_selection: action.payload,
        },
      };

    case "SET_LOADING":
      return {
        ...state,
        isLoading: action.payload,
      };

    case "INVALIDATE_CACHE":
      return {
        ...state,
        settings: null,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface SettingsContextType {
  state: SettingsState;
  dispatch: React.Dispatch<SettingsAction>;
}

/**
 * Settings Context
 */
const SettingsContext = createContext<SettingsContextType | undefined>(
  undefined,
);

/**
 * Settings Provider Props
 */
interface SettingsProviderProps {
  children: ReactNode;
}

/**
 * Settings Provider Component
 * Provides shared settings state across the application
 */
export function SettingsProvider({ children }: SettingsProviderProps) {
  const [state, dispatch] = useReducer(settingsReducer, initialState);

  return (
    <SettingsContext.Provider value={{ state, dispatch }}>
      {children}
    </SettingsContext.Provider>
  );
}

/**
 * Hook to access settings context
 * Must be used within a SettingsProvider
 */
export function useSettingsContext(): SettingsContextType {
  const context = useContext(SettingsContext);

  if (!context) {
    throw new Error(
      "useSettingsContext must be used within a SettingsProvider",
    );
  }

  return context;
}
