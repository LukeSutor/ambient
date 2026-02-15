"use client";

import { useModelAccess } from "@/lib/model-access";
import { useSettings } from "@/lib/settings/useSettings";
import { cn } from "@/lib/utils";
import { useMemo } from "react";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "../ui/hover-card";
import { Progress } from "../ui/progress";

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
    models,
    dailyCreditLimit,
    isHydrated,
    usagePercent,
  } = useModelAccess();

  const { settings } = useSettings();
  const modelSelection = settings?.model_selection ?? "1";
  const isCloudModelSelected = useMemo(() => {
    const selectedModel = models.find(
      (m) => m.id.toString() === modelSelection,
    );
    return selectedModel?.is_cloud && selectedModel.is_internal;
  }, [modelSelection, models]);

  const isUnlimited = creditsRemaining === -1;
  const hasCredits = dailyCreditLimit > 0;
  const shouldShow =
    isHydrated && !isUnlimited && hasCredits && isCloudModelSelected;

  const remainingPercent = Math.max(0, 100 - usagePercent);

  // Color based on remaining percentage
  const barColor = useMemo(() => {
    if (remainingPercent > 50) return "[&>*]:bg-emerald-500/70";
    if (remainingPercent > 20) return "[&>*]:bg-amber-500/70";
    return "[&>*]:bg-red-500/70";
  }, [remainingPercent]);

  const textColor = useMemo(() => {
    if (remainingPercent > 50) return "text-emerald-700/70";
    if (remainingPercent > 20) return "text-amber-700/70";
    return "text-red-700/70";
  }, [remainingPercent]);

  if (!shouldShow) return <div className="ml-auto" />;

  const displayRemaining = Math.max(0, creditsRemaining);

  return (
    <HoverCard openDelay={250} closeDelay={100}>
      <HoverCardTrigger className="ml-auto cursor-auto p-2 ">
        <Progress
          value={remainingPercent}
          className={cn("w-12 h-1", barColor)}
        />
      </HoverCardTrigger>
      <HoverCardContent className="w-min whitespace-nowrap p-2 gap-y-1 space-y-1">
        <div className="flex flex-col space-y-1 text-xs">
          <p>
            Credits available:{" "}
            <span className={cn("font-bold", textColor)}>
              {remainingPercent}%
            </span>
          </p>
          <p className="text-muted-foreground">Credits reset daily.</p>
        </div>
      </HoverCardContent>
    </HoverCard>
  );
}
