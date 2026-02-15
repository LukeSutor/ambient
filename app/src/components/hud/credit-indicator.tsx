"use client";

import { useModelAccess } from "@/lib/model-access";
import { cn } from "@/lib/utils";
import { useMemo } from "react";

/**
 * Compact credit usage indicator for the HUD.
 *
 * Shows a thin progress bar with remaining credit count.
 * Only visible when the user is signed in and has cloud models available.
 * Colors shift from green → yellow → red as usage increases.
 */
export function CreditIndicator() {
  const {
    creditsRemaining,
    dailyCreditLimit,
    isHydrated,
    usagePercent,
  } = useModelAccess();

  const isUnlimited = creditsRemaining === -1;
  const hasCredits = dailyCreditLimit > 0;

  // Don't show if not hydrated, unlimited, or no credit system
  if (!isHydrated || isUnlimited || !hasCredits) return null;

  const remainingPercent = Math.max(0, 100 - usagePercent);

  // Color based on remaining percentage
  const barColor = useMemo(() => {
    if (remainingPercent > 50) return "bg-emerald-500/70";
    if (remainingPercent > 20) return "bg-amber-500/70";
    return "bg-red-500/70";
  }, [remainingPercent]);

  const textColor = useMemo(() => {
    if (remainingPercent > 50) return "text-emerald-700/70";
    if (remainingPercent > 20) return "text-amber-700/70";
    return "text-red-700/70";
  }, [remainingPercent]);

  const displayRemaining = Math.max(0, creditsRemaining);

  return (
    <div className="flex items-center gap-2 px-1">
      {/* Thin progress bar */}
      <div className="flex-1 h-1 rounded-full bg-black/5 overflow-hidden min-w-12">
        <div
          className={cn("h-full rounded-full transition-all duration-500", barColor)}
          style={{ width: `${remainingPercent}%` }}
        />
      </div>

      {/* Credit count */}
      <span className={cn("text-[10px] font-medium tabular-nums whitespace-nowrap", textColor)}>
        {displayRemaining}/{dailyCreditLimit}
      </span>
    </div>
  );
}
