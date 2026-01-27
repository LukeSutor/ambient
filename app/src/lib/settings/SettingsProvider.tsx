"use client";

import type {
  HudSizeOption,
  ModelSelection,
  UserSettings,
} from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import type React from "react";
import {
  type ReactNode,
  type RefObject,
  createContext,
  useContext,
  useEffect,
  useReducer,
  useRef,
} from "react";

/**
 * Settings state
 */
interface SettingsState {
  settings: UserSettings | null;
  isLoading: boolean;
  initializationRef: RefObject<boolean>;
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

  // Initialization effect
  useEffect(() => {
    if (state.initializationRef.current || state.settings) {
      return;
    }
    state.initializationRef.current = true;

    const initialize = async () => {
      console.log("[SettingsProvider] Initializing settings...");

      try {
        dispatch({ type: "SET_LOADING", payload: true });
        const settings = await invoke<UserSettings>("load_user_settings");
        dispatch({ type: "SET_SETTINGS", payload: settings });
      } catch (error) {
        console.error("[SettingsProvider] Failed to load settings:", error);

        const defaults: UserSettings = {
          hud_size: "Normal",
          model_selection: "Local",
        };
        dispatch({ type: "SET_SETTINGS", payload: defaults });
      }
    };

    void initialize();
  }, [state.initializationRef, state.settings]);

  // Event listeners setup
  useEffect(() => {
    let isMounted = true;
    let cleanup: UnlistenFn | null = null;

    const setupEvents = async () => {
      if (!isMounted) return;

      try {
        console.log("[SettingsProvider] Setting up event listeners...");

        cleanup = await listen("settings_changed", () => {
          void (async () => {
            console.log(
              "[SettingsProvider] Settings changed event received, reloading",
            );

            dispatch({ type: "INVALIDATE_CACHE" });

            if (isMounted) {
              try {
                const settings =
                  await invoke<UserSettings>("load_user_settings");
                dispatch({ type: "SET_SETTINGS", payload: settings });
              } catch (error) {
                console.error(
                  "[SettingsProvider] Failed to reload settings:",
                  error,
                );
              }
            }
          })();
        });

        console.log("[SettingsProvider] Event listeners initialized");
      } catch (error) {
        console.error("[SettingsProvider] Failed to setup events:", error);
      }
    };

    void setupEvents();

    return () => {
      isMounted = false;
      if (cleanup) {
        cleanup();
      }
      console.log("[SettingsProvider] Event listeners cleaned up");
    };
  }, []);

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
