"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InputGroupButton } from "@/components/ui/input-group";
import { useSettings } from "@/lib/settings";
import type { CloudModelUsage, ModelEntry } from "@/types/models";
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
  const [cloudUsage, setCloudUsage] = useState<Record<string, CloudModelUsage>>({});

  const fetchModels = useCallback(async () => {
    try {
      const result = await invoke<ModelEntry[]>("get_models");
      setModels(result);
    } catch (e) {
      console.error("Failed to fetch models:", e);
    }
  }, []);

  const fetchCloudUsage = useCallback(async () => {
    try {
      const result = await invoke<Record<string, CloudModelUsage>>("get_remaining_cloud_uses");
      setCloudUsage(result);
    } catch (e) {
      // Silently fail — user may not be signed in
      console.debug("Failed to fetch cloud usage:", e);
    }
  }, []);

  useEffect(() => {
    fetchModels();
    fetchCloudUsage();
  }, [fetchModels, fetchCloudUsage]);

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
          {models.map((model) => {
            const usage = model.is_cloud ? cloudUsage[model.model] : undefined;
            const isAtLimit = model.is_cloud && !model.is_premium && usage && usage.remaining <= 0;
            const isDisabled = model.is_premium || !!isAtLimit;

            // Build the subtitle line
            let subtitle: { text: string; className: string };
            if (isAtLimit) {
              subtitle = { text: "No usage left today", className: "text-xs text-destructive" };
            } else if (model.is_cloud && !model.is_premium && usage) {
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
