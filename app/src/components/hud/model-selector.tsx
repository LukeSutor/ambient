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
import type { ModelEntry } from "@/types/models";
import { ChevronDown } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useMemo, useState } from "react";

interface ModelSelectorProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
}

export function ModelSelector({ onOpenChange, disabled }: ModelSelectorProps) {
  const { settings, setModelSelection } = useSettings();
  const modelSelection = settings?.model_selection ?? "qwen3vl-2b";
  const [models, setModels] = useState<ModelEntry[]>([]);
  const { cloudUsage } = useModelAccess();

  const fetchModels = useCallback(async () => {
    try {
      const result = await invoke<ModelEntry[]>("get_models");
      setModels(result);
    } catch (e) {
      console.error("Failed to fetch models:", e);
    }
  }, []);

  useEffect(() => {
    fetchModels();
  }, [fetchModels]);

  const handleModelSelectionChange = useCallback(
    async (modelKey: string) => {
      try {
        await setModelSelection(modelKey);
      } catch (error) {
        console.error("Failed to save model selection setting:", error);
      }
    },
    [setModelSelection],
  );

  const currentLabel = useMemo(() => {
    const model = models.find((m) => m.model === modelSelection);
    return model?.display_name ?? "Local";
  }, [modelSelection, models]);

  return (
    <DropdownMenu onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <InputGroupButton
          className="ml-auto"
          variant="ghost"
          disabled={disabled}
        >
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
          {models.filter((m) => m.is_enabled).map((model) => {
            const usage = model.is_cloud ? cloudUsage[model.model] : undefined;
            const isUnlimited = usage?.remaining === -1;
            const isUnavailable = usage?.is_available === false;
            const isAtLimit = !isUnavailable && !isUnlimited && usage?.is_available && usage.remaining <= 0;
            const isDisabled = isUnavailable || !!isAtLimit;

            // Build the subtitle line
            let subtitle: { text: string; className: string };
            if (isUnavailable) {
              subtitle = { text: "Upgrade to unlock", className: "text-xs text-muted-foreground" };
            } else if (isAtLimit) {
              subtitle = { text: "No usage left today", className: "text-xs text-destructive" };
            } else if (model.is_cloud && usage && !isUnlimited) {
              subtitle = {
                text: `${model.short_description} ${usage.remaining}/${usage.daily_limit} left today`,
                className: "text-xs text-muted-foreground",
              };
            } else {
              subtitle = { text: model.short_description, className: "text-xs text-muted-foreground" };
            }

            return (
              <DropdownMenuItem
                key={model.model}
                onClick={() => !isDisabled && void handleModelSelectionChange(model.model)}
                className={`py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60 ${isDisabled ? "opacity-50 pointer-events-none" : ""}`}
                disabled={isDisabled}
              >
                <span className="font-medium text-sm">{model.display_name}</span>
                <span className={subtitle.className}>{subtitle.text}</span>
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
