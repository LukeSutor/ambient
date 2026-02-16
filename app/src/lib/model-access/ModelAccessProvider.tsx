"use client";

import type { CreditUsageResponse, ModelEntry } from "@/types/models";
import { invoke } from "@tauri-apps/api/core";
import { type UnlistenFn, listen } from "@tauri-apps/api/event";
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
  dailyCreditLimit: 10,
  creditsUsed: 0,
  creditsRemaining: 10,
  modelCosts: {},
  models: [],
  isHydrated: false,
};

type ModelAccessAction =
  | {
      type: "SET_CREDIT_DATA";
      payload: {
        userTier: "free" | "premium" | "admin";
        dailyCreditLimit: number;
        creditsUsed: number;
        creditsRemaining: number;
        modelCosts: Record<string, number>;
      };
    }
  | { type: "DECREMENT_CREDITS"; payload: { creditCost: number } }
  | { type: "SET_MODELS"; payload: { models: ModelEntry[] } }
  | { type: "SET_HYDRATED" };

function modelAccessReducer(
  state: ModelAccessState,
  action: ModelAccessAction,
): ModelAccessState {
  switch (action.type) {
    case "SET_CREDIT_DATA":
      return {
        ...state,
        userTier: action.payload.userTier,
        dailyCreditLimit: action.payload.dailyCreditLimit,
        creditsUsed: action.payload.creditsUsed,
        creditsRemaining: action.payload.creditsRemaining,
        modelCosts: action.payload.modelCosts,
        isHydrated: true,
      };

    case "DECREMENT_CREDITS": {
      const { creditCost } = action.payload;
      if (state.creditsRemaining === -1) return state; // unlimited
      return {
        ...state,
        creditsUsed: state.creditsUsed + creditCost,
        creditsRemaining: Math.max(0, state.creditsRemaining - creditCost),
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
  /** Refresh credit usage data from the Cloudflare backend. */
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
      const result = await invoke<CreditUsageResponse>("get_credit_usage");
      console.log("Fetched credit usage data:", result);
      dispatch({
        type: "SET_CREDIT_DATA",
        payload: {
          userTier: result.user_tier as "free" | "premium" | "admin",
          dailyCreditLimit: result.daily_credit_limit,
          creditsUsed: result.credits_used,
          creditsRemaining: result.credits_remaining,
          modelCosts: result.model_costs as Record<string, number>,
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
      unlistenUsage = await listen<{ credit_cost: number }>(
        "cloud_usage_decremented",
        (event) => {
          dispatch({
            type: "DECREMENT_CREDITS",
            payload: { creditCost: event.payload.credit_cost },
          });
        },
      );

      // Re-fetch when auth state changes (login/logout/role change)
      unlistenAuth = await listen("auth_changed", () => {
        void refreshUsage();
        void refreshModels();
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
