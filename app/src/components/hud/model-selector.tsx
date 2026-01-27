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
import type { ModelSelection } from "@/types/settings";
import { ChevronDown } from "lucide-react";
import { useCallback, useMemo } from "react";

interface ModelSelectorProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
}

const MODEL_OPTIONS = [
  {
    value: "Local",
    label: "Local",
    description: "Ultimate privacy. Runs on your device.",
  },
  {
    value: "Fast",
    label: "Gemini 3 Flash",
    description: "More powerful fast model.",
  },
  {
    value: "Pro",
    label: "Gemini 3 Pro",
    description: "The latest and most advanced model.",
  },
] as const;

export function ModelSelector({ onOpenChange, disabled }: ModelSelectorProps) {
  const { settings, setModelSelection } = useSettings();
  const modelSelection = settings?.model_selection ?? "Local";

  const handleModelSelectionChange = useCallback(
    async (value: ModelSelection) => {
      try {
        await setModelSelection(value);
      } catch (error) {
        console.error("Failed to save model selection setting:", error);
      }
    },
    [setModelSelection],
  );

  const currentLabel = useMemo(
    () =>
      MODEL_OPTIONS.find((opt) => opt.value === modelSelection)?.label ??
      modelSelection,
    [modelSelection],
  );

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
        alignOffset={-115}
        className="w-full bg-white/60"
      >
        <DropdownMenuGroup>
          {MODEL_OPTIONS.map((option) => (
            <DropdownMenuItem
              key={option.value}
              onClick={() => void handleModelSelectionChange(option.value)}
              className="py-1.5 px-2 cursor-pointer flex-col gap-0.5 items-start hover:bg-white/60"
            >
              <span className="font-medium text-sm">{option.label}</span>
              <span className="text-xs text-muted-foreground">
                {option.description}
              </span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
