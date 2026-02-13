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
        align="start"
        avoidCollisions={false}
        sideOffset={10}
        alignOffset={-185}
        className="w-full bg-white/60"
      >
        <DropdownMenuGroup>
          {models.map((model) => (
            <DropdownMenuItem
              key={model.model}
              onClick={() => void handleModelSelectionChange(model.model)}
              className="py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60"
            >
              <span className="font-medium text-sm">{model.display_name}</span>
              <span className="text-xs text-muted-foreground">
                {model.description}
              </span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
