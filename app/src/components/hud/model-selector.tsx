"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InputGroupButton } from "@/components/ui/input-group";
import { useModelAccess } from "@/lib/model-access";
import { useSettings } from "@/lib/settings";
import { cn } from "@/lib/utils";
import { ChevronDown } from "lucide-react";
import { useCallback, useMemo } from "react";

interface ModelSelectorProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
}

export function ModelSelector({ onOpenChange, disabled }: ModelSelectorProps) {
  const { settings, setModelSelection } = useSettings();
  const modelSelection = settings?.model_selection ?? "1";
  const {
    enabledModels,
    canAffordModel,
    modelCosts,
    creditsRemaining,
    dailyCreditLimit,
  } = useModelAccess();

  const handleModelSelectionChange = useCallback(
    async (modelId: string) => {
      try {
        await setModelSelection(modelId);
      } catch (error) {
        console.error("Failed to save model selection setting:", error);
      }
    },
    [setModelSelection],
  );

  const currentLabel = useMemo(() => {
    const model = enabledModels.find((m) => m.id.toString() === modelSelection);
    return model?.display_name ?? "Local";
  }, [modelSelection, enabledModels]);

  return (
    <DropdownMenu onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <InputGroupButton variant="ghost" disabled={disabled}>
          {currentLabel}
          <ChevronDown />
        </InputGroupButton>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="bottom"
        align="end"
        avoidCollisions={false}
        sideOffset={10}
        className="min-w-48 bg-white/60"
      >
        <DropdownMenuGroup>
          {enabledModels.map((model) => {
            const cost =
              model.is_cloud && model.is_internal
                ? modelCosts[model.model]
                : undefined;
            const isAffordable = canAffordModel(model.model);
            const isDisabled =
              model.is_cloud && model.is_internal && !isAffordable;

            // Build the subtitle and credits line
            let subtitle: { text: string; credits: string; className: string };
            if (isDisabled) {
              subtitle = {
                text: `Needs ${cost} credit${cost !== 1 ? "s" : ""} — not enough remaining`,
                credits: "",
                className: "text-xs text-destructive",
              };
            } else if (
              model.is_cloud &&
              model.is_internal &&
              cost !== undefined
            ) {
              subtitle = {
                text: model.short_description,
                credits: `${cost}x`,
                className: "text-xs text-muted-foreground",
              };
            } else {
              subtitle = {
                text: model.short_description,
                credits: "",
                className: "text-xs text-muted-foreground",
              };
            }

            return (
              <DropdownMenuItem
                key={model.id}
                onClick={() =>
                  !isDisabled &&
                  void handleModelSelectionChange(model.id.toString())
                }
                className={`py-1.5 px-2 cursor-pointer flex-col gap-0 items-start hover:bg-white/60 ${isDisabled ? "opacity-50 pointer-events-none" : ""}`}
                disabled={isDisabled}
              >
                <span className="font-medium text-sm">
                  {model.display_name}
                </span>
                <div
                  className={cn(
                    "flex flex-row justify-between w-full",
                    subtitle.className,
                  )}
                >
                  <span>{subtitle.text}</span>
                  <span>{subtitle.credits}</span>
                </div>
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
