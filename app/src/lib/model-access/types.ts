import type { CloudModelUsage } from "@/types/models";

/**
 * Centralized model access state.
 *
 * Tracks the user's effective tier and cloud model usage (remaining
 * daily uses). Provides a single source of truth that updates in
 * real time as cloud generations complete.
 *
 * Future extensions:
 * - Premium subscription details (renewal date, cancel_at_period_end)
 * - Custom model limits overrides per user
 */
export interface ModelAccessState {
  /** The user's effective tier: "free", "premium", or "admin". */
  userTier: "free" | "premium" | "admin";
  /** Cloud usage data keyed by model key (e.g. "gemini-3-flash"). */
  cloudUsage: Record<string, CloudModelUsage>;
  /** Whether the initial usage fetch has completed. */
  isHydrated: boolean;
}
