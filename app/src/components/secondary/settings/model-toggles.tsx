"use client";

import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Field } from "@/components/ui/field";
import { Label } from "@/components/ui/label";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverHeader,
  PopoverTrigger,
} from "@/components/ui/popover";
import { useModelAccess } from "@/lib/model-access";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useMemo } from "react";
import { toast } from "sonner";

/**
 * ModelVisibilityPopover renders a ghost button that opens a popover
 * with checkboxes for toggling model visibility in the HUD model selector.
 *
 * Uses the centralized ModelAccessProvider for the model list.
 * Toggling a model emits a `models_changed` event from Rust,
 * which the provider listens to for real-time updates.
 *
 * The toggle logic is tier-aware: only models the user can access
 * count toward the "at least one enabled" requirement, and auto-
 * reselect only picks from accessible models.
 */
export function ModelVisibilityPopover() {
  const { models, isModelAvailable } = useModelAccess();

  /** Model keys the user can actually access (based on tier). */
  const allowedModelKeys = useMemo(
    () => models.filter((m) => isModelAvailable(m.model)).map((m) => m.model),
    [models, isModelAvailable],
  );

  /** Number of enabled models that the user can access. */
  const allowedEnabledCount = useMemo(
    () =>
      models.filter((m) => m.is_enabled && isModelAvailable(m.model)).length,
    [models, isModelAvailable],
  );

  const handleToggle = useCallback(
    async (modelKey: string, currentEnabled: boolean) => {
      const newEnabled = !currentEnabled;
      try {
        await invoke<string | null>("toggle_model", {
          modelKey,
          enabled: newEnabled,
          allowedModels: allowedModelKeys,
        });
      } catch (error) {
        const msg = String(error);
        if (msg.includes("Cannot disable the last enabled model")) {
          toast.error("At least one model must remain enabled");
        } else {
          console.error(`[ModelVisibilityPopover] Failed to toggle model ${modelKey}:`, error);
          toast.error("Failed to toggle model");
        }
      }
    },
    [allowedModelKeys],
  );

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant="ghost" size="sm" className="h-auto px-1 py-0 text-xs text-muted-foreground">
          Configure
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start">
        <PopoverHeader>
          <PopoverDescription>The checked models will be visible in the model selection dropdowns.</PopoverDescription>
        </PopoverHeader>
        <div className="flex flex-col gap-3 mt-4">
          {models.map((model) => {
            const isAllowed = isModelAvailable(model.model);
            // Disable checkbox if this is the last allowed-and-enabled model
            const isLastAllowedEnabled =
              model.is_enabled && isAllowed && allowedEnabledCount <= 1;
            return (
              <Field key={model.model} orientation="horizontal">
                <Checkbox
                  id={`model-${model.model}`}
                  checked={model.is_enabled}
                  disabled={isLastAllowedEnabled}
                  onCheckedChange={() =>
                    void handleToggle(model.model, model.is_enabled)
                  }
                />
                <Label htmlFor={`model-${model.model}`} className="w-full">{model.display_name}</Label>
              </Field>
            );
          })}
        </div>
      </PopoverContent>
    </Popover>
  );
}
