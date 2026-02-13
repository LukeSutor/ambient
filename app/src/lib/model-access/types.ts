import type { CloudModelUsage } from "@/types/models";

/**
 * Centralized model access state.
 *
 * Tracks cloud model usage (remaining daily uses) and provides a single
 * source of truth that updates in real time as cloud generations complete.
 *
 * Future extensions:
 * - `userTier: "free" | "premium" | "admin"` to determine access policies
 * - `modelLimits: Record<string, number>` overridden by subscription plan
 * - `premiumModels: string[]` list of models unlocked by premium plans
 */
export interface ModelAccessState {
  /** Cloud usage data keyed by model key (e.g. "gemini-3-flash"). */
  cloudUsage: Record<string, CloudModelUsage>;
  /** Whether the initial usage fetch has completed. */
  isHydrated: boolean;
}
