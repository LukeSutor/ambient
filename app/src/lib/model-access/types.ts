import type { CloudModelUsage, ModelEntry } from "@/types/models";

/**
 * Centralized model access state.
 *
 * Tracks the user's effective tier, cloud model usage (remaining
 * daily uses), and the full list of registered models. Provides a
 * single source of truth that updates in real time as cloud
 * generations complete or model visibility is toggled.
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
  /** All registered models from the local database. */
  models: ModelEntry[];
  /** Whether the initial usage fetch has completed. */
  isHydrated: boolean;
}
