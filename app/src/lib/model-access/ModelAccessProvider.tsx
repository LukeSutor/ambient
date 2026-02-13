"use client";

import type { CloudModelUsage } from "@/types/models";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import type React from "react";
import {
  type ReactNode,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  useRef,
} from "react";
import type { ModelAccessState } from "./types";

// ---------------------------------------------------------------------------
// Reducer
// ---------------------------------------------------------------------------

const initialState: ModelAccessState = {
  cloudUsage: {},
  isHydrated: false,
};

type ModelAccessAction =
  | { type: "SET_CLOUD_USAGE"; payload: Record<string, CloudModelUsage> }
  | { type: "DECREMENT_REMAINING"; payload: { modelKey: string } }
  | { type: "SET_HYDRATED" };

function modelAccessReducer(
  state: ModelAccessState,
  action: ModelAccessAction,
): ModelAccessState {
  switch (action.type) {
    case "SET_CLOUD_USAGE":
      return { ...state, cloudUsage: action.payload, isHydrated: true };

    case "DECREMENT_REMAINING": {
      const { modelKey } = action.payload;
      const current = state.cloudUsage[modelKey];
      if (!current) return state;
      return {
        ...state,
        cloudUsage: {
          ...state.cloudUsage,
          [modelKey]: {
            ...current,
            requests_used: current.requests_used + 1,
            remaining: Math.max(0, current.remaining - 1),
          },
        },
      };
    }

    case "SET_HYDRATED":
      return { ...state, isHydrated: true };

    default:
      return state;
  }
}

// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------

interface ModelAccessContextType {
  state: ModelAccessState;
  /** Refresh usage data from the Cloudflare backend. */
  refreshUsage: () => Promise<void>;
}

const ModelAccessContext = createContext<ModelAccessContextType | undefined>(
  undefined,
);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

interface ModelAccessProviderProps {
  children: ReactNode;
}

export function ModelAccessProvider({ children }: ModelAccessProviderProps) {
  const [state, dispatch] = useReducer(modelAccessReducer, initialState);
  const isFetching = useRef(false);

  const refreshUsage = useCallback(async () => {
    if (isFetching.current) return;
    isFetching.current = true;

    try {
      const result = await invoke<Record<string, CloudModelUsage>>(
        "get_remaining_cloud_uses",
      );
      dispatch({ type: "SET_CLOUD_USAGE", payload: result });
    } catch {
      // User may not be signed in — silently hydrate with empty data
      dispatch({ type: "SET_HYDRATED" });
    } finally {
      isFetching.current = false;
    }
  }, []);

  useEffect(() => {
    // Initial fetch
    void refreshUsage();

    // Listen for real-time usage updates from the Rust backend
    let unlistenUsage: UnlistenFn | undefined;
    let unlistenAuth: UnlistenFn | undefined;

    const subscribe = async () => {
      unlistenUsage = await listen<{ model_key: string }>(
        "cloud_usage_decremented",
        (event) => {
          dispatch({
            type: "DECREMENT_REMAINING",
            payload: { modelKey: event.payload.model_key },
          });
        },
      );

      // Re-fetch when auth state changes (login/logout)
      unlistenAuth = await listen("auth_changed", () => {
        void refreshUsage();
      });
    };

    void subscribe();

    return () => {
      unlistenUsage?.();
      unlistenAuth?.();
    };
  }, [refreshUsage]);

  return (
    <ModelAccessContext.Provider value={{ state, refreshUsage }}>
      {children}
    </ModelAccessContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useModelAccessContext(): ModelAccessContextType {
  const context = useContext(ModelAccessContext);
  if (!context) {
    throw new Error(
      "useModelAccessContext must be used within a ModelAccessProvider",
    );
  }
  return context;
}
