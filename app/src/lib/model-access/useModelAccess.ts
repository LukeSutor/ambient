"use client";

import type { CloudModelUsage } from "@/types/models";
import { useModelAccessContext } from "./ModelAccessProvider";

/**
 * Hook to access cloud model usage data and helper functions.
 *
 * Returns the centralized usage state that updates in real time
 * as cloud generations complete, plus utility functions for
 * checking model availability.
 */
export function useModelAccess() {
  const { state, refreshUsage } = useModelAccessContext();

  return {
    /** The user's effective tier: "free", "premium", or "admin". */
    userTier: state.userTier,
    /** Cloud usage keyed by model key. */
    cloudUsage: state.cloudUsage,
    /** Whether the initial fetch has completed. */
    isHydrated: state.isHydrated,
    /** Force-refresh usage data from the backend. */
    refreshUsage,
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
