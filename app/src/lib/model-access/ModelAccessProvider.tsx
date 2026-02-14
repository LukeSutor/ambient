"use client";

import type { CloudModelUsage, ModelAccessResponse, ModelEntry } from "@/types/models";
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
  userTier: "free",
  cloudUsage: {},
  models: [],
  isHydrated: false,
};

type ModelAccessAction =
  | { type: "SET_ACCESS_DATA"; payload: { userTier: "free" | "premium" | "admin"; cloudUsage: Record<string, CloudModelUsage> } }
  | { type: "DECREMENT_REMAINING"; payload: { modelKey: string } }
  | { type: "SET_MODELS"; payload: { models: ModelEntry[] } }
  | { type: "SET_HYDRATED" };

function modelAccessReducer(
  state: ModelAccessState,
  action: ModelAccessAction,
): ModelAccessState {
  switch (action.type) {
    case "SET_ACCESS_DATA":
      return {
        ...state,
        userTier: action.payload.userTier,
        cloudUsage: action.payload.cloudUsage,
        isHydrated: true,
      };

    case "DECREMENT_REMAINING": {
      const { modelKey } = action.payload;
      const current = state.cloudUsage[modelKey];
      if (!current || current.remaining === -1) return state; // unlimited
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

    case "SET_MODELS":
      return { ...state, models: action.payload.models };

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
  /** Refresh the model list from the local database. */
  refreshModels: () => Promise<void>;
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

  const refreshModels = useCallback(async () => {
    try {
      const models = await invoke<ModelEntry[]>("get_models");
      dispatch({ type: "SET_MODELS", payload: { models } });
    } catch (e) {
      console.error("[ModelAccessProvider] Failed to fetch models:", e);
    }
  }, []);

  const refreshUsage = useCallback(async () => {
    if (isFetching.current) return;
    isFetching.current = true;

    try {
      const result = await invoke<ModelAccessResponse>(
        "get_remaining_cloud_uses",
      );
      console.log("Fetched model access data:", result);
      dispatch({
        type: "SET_ACCESS_DATA",
        payload: {
          userTier: result.user_tier as "free" | "premium" | "admin",
          cloudUsage: result.models as Record<string, CloudModelUsage>,
        },
      });
    } catch {
      // User may not be signed in — silently hydrate with empty data
      dispatch({ type: "SET_HYDRATED" });
    } finally {
      isFetching.current = false;
    }
  }, []);

  useEffect(() => {
    // Initial fetch — models from local DB + usage from cloud
    void refreshModels();
    void refreshUsage();

    // Listen for real-time updates from the Rust backend
    let unlistenUsage: UnlistenFn | undefined;
    let unlistenAuth: UnlistenFn | undefined;
    let unlistenModels: UnlistenFn | undefined;

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

      // Re-fetch when auth state changes (login/logout/role change)
      unlistenAuth = await listen("auth_changed", () => {
        void refreshUsage();
      });

      // Re-fetch models when visibility is toggled
      unlistenModels = await listen("models_changed", () => {
        void refreshModels();
      });
    };

    void subscribe();

    return () => {
      unlistenUsage?.();
      unlistenAuth?.();
      unlistenModels?.();
    };
  }, [refreshUsage, refreshModels]);

  return (
    <ModelAccessContext.Provider value={{ state, refreshUsage, refreshModels }}>
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
