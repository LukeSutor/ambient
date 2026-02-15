"use client";

import { useModelAccessContext } from "./ModelAccessProvider";
import { useMemo } from "react";

/**
 * Hook to access credit usage data, the model list, and helper functions.
 *
 * Returns the centralized credit state that updates in real time
 * as cloud generations complete or model visibility is toggled,
 * plus utility functions for checking model affordability.
 */
export function useModelAccess() {
  const { state, refreshUsage, refreshModels } = useModelAccessContext();

  /** Models filtered to only those the user has enabled. */
  const enabledModels = useMemo(
    () => state.models.filter((m) => m.is_enabled),
    [state.models],
  );

  return {
    /** The user's effective tier: "free", "premium", or "admin". */
    userTier: state.userTier,
    /** Daily credit limit. -1 means unlimited. */
    dailyCreditLimit: state.dailyCreditLimit,
    /** Credits consumed today. */
    creditsUsed: state.creditsUsed,
    /** Credits remaining today. -1 means unlimited. */
    creditsRemaining: state.creditsRemaining,
    /** Per-model credit costs keyed by model key. */
    modelCosts: state.modelCosts,
    /** All registered models (enabled and disabled). */
    models: state.models,
    /** Only models the user has enabled (for selectors). */
    enabledModels,
    /** Whether the initial fetch has completed. */
    isHydrated: state.isHydrated,
    /** Force-refresh credit usage data from the backend. */
    refreshUsage,
    /** Force-refresh the model list from the local database. */
    refreshModels,
    /** Get the credit cost for an internal cloud model, or undefined for local/BYOK models. */
    getModelCost: (modelKey: string): number | undefined =>
      state.modelCosts[modelKey],
    /**
     * Check if the user can afford to use a given model.
     * Local and BYOK models are always affordable.
     * Returns false if the model's credit cost exceeds remaining credits.
     */
    canAffordModel: (modelKey: string): boolean => {
      if (state.creditsRemaining === -1) return true; // unlimited
      const cost = state.modelCosts[modelKey];
      if (cost === undefined) return true; // local/BYOK — no credit cost
      return state.creditsRemaining >= cost;
    },
    /** Usage as a percentage (0–100). Returns 0 for unlimited users. */
    usagePercent: state.creditsRemaining === -1
      ? 0
      : state.dailyCreditLimit > 0
        ? Math.round((state.creditsUsed / state.dailyCreditLimit) * 100)
        : 0,
  };
}
