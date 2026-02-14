"use client";

import type { CloudModelUsage } from "@/types/models";
import { useModelAccessContext } from "./ModelAccessProvider";
import { useMemo } from "react";

/**
 * Hook to access cloud model usage data, the model list, and helper
 * functions.
 *
 * Returns the centralized usage state that updates in real time
 * as cloud generations complete or model visibility is toggled,
 * plus utility functions for checking model availability.
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
    /** Cloud usage keyed by model key. */
    cloudUsage: state.cloudUsage,
    /** All registered models (enabled and disabled). */
    models: state.models,
    /** Only models the user has enabled (for selectors). */
    enabledModels,
    /** Whether the initial fetch has completed. */
    isHydrated: state.isHydrated,
    /** Force-refresh usage data from the backend. */
    refreshUsage,
    /** Force-refresh the model list from the local database. */
    refreshModels,
    /** Get usage info for a specific model, or undefined if not a cloud model. */
    getUsage: (modelKey: string): CloudModelUsage | undefined =>
      state.cloudUsage[modelKey],
    /** Check if a cloud model is available for the current user's tier. */
    isModelAvailable: (modelKey: string): boolean => {
      const usage = state.cloudUsage[modelKey];
      if (!usage) return true; // local models are always available
      return usage.is_available;
    },
  };
}
