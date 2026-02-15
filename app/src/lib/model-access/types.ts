import type { ModelEntry } from "@/types/models";

/**
 * Centralized model access state.
 *
 * Tracks the user's effective tier, global credit usage, and the full
 * list of registered models. Provides a single source of truth that
 * updates in real time as cloud generations complete or model visibility
 * is toggled.
 *
 * The credit system charges a global pool per day — all cloud model usage
 * counts toward the same limit. Different models cost different amounts
 * (e.g. Flash = 1 credit, Pro = 3 credits).
 */
export interface ModelAccessState {
  /** The user's effective tier: "free", "premium", or "admin". */
  userTier: "free" | "premium" | "admin";
  /** Daily credit limit. -1 means unlimited. */
  dailyCreditLimit: number;
  /** Credits consumed today. */
  creditsUsed: number;
  /** Credits remaining today. -1 means unlimited. */
  creditsRemaining: number;
  /** Per-model credit costs keyed by model key (e.g. "gemini-3-flash" → 1). */
  modelCosts: Record<string, number>;
  /** All registered models from the local database. */
  models: ModelEntry[];
  /** Whether the initial usage fetch has completed. */
  isHydrated: boolean;
}
