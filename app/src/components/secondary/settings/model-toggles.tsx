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
import type { ModelEntry } from "@/types/models";
import { invoke } from "@tauri-apps/api/core";
import { Pencil, Plus } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import { toast } from "sonner";
import { ModelDialog } from "./model-dialog";

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
  const { models } = useModelAccess();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editingModel, setEditingModel] = useState<ModelEntry | null>(null);

  /** Number of enabled models. */
  const enabledCount = useMemo(
    () => models.filter((m) => m.is_enabled).length,
    [models],
  );

  const handleToggle = useCallback(
    async (modelId: number, currentEnabled: boolean) => {
      const newEnabled = !currentEnabled;
      try {
        await invoke<string | null>("toggle_model", {
          modelId,
          enabled: newEnabled,
        });
      } catch (error) {
        const msg = String(error);
        if (msg.includes("Cannot disable the last enabled model")) {
          toast.error("At least one model must remain enabled");
        } else {
          console.error(`[ModelVisibilityPopover] Failed to toggle model ${modelId}:`, error);
          toast.error("Failed to toggle model");
        }
      }
    },
    [],
  );

  const handleAddModel = useCallback(() => {
    setEditingModel(null);
    setDialogOpen(true);
  }, []);

  const handleEditModel = useCallback((model: ModelEntry) => {
    setEditingModel(model);
    setDialogOpen(true);
  }, []);

  return (
    <>
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
              // Disable checkbox if this is the last enabled model
              const isLastEnabled =
                model.is_enabled && enabledCount <= 1;
              return (
                <Field key={model.id} orientation="horizontal">
                  <Checkbox
                    id={`model-${model.id}`}
                    checked={model.is_enabled}
                    disabled={isLastEnabled}
                    onCheckedChange={() =>
                      void handleToggle(model.id, model.is_enabled)
                    }
                  />
                  <Label htmlFor={`model-${model.id}`} className="w-full">{model.display_name}</Label>
                  {!model.is_internal && (
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-6 w-6 p-0 shrink-0"
                      onClick={() => handleEditModel(model)}
                    >
                      <Pencil className="h-3 w-3" />
                    </Button>
                  )}
                </Field>
              );
            })}
          </div>
          <Button
            variant="outline"
            size="sm"
            className="w-full mt-4"
            onClick={handleAddModel}
          >
            <Plus className="h-3 w-3 mr-1" />
            Add Model
          </Button>
        </PopoverContent>
      </Popover>

      <ModelDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        model={editingModel}
      />
    </>
  );
}
