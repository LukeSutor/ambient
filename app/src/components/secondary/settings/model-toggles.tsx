"use client";

import { Switch } from "@/components/ui/switch";
import type { ModelEntry } from "@/types/models";
import { Crown, Shield, Zap } from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";

const ICON_MAP: Record<string, React.ElementType> = {
  shield: Shield,
  zap: Zap,
  crown: Crown,
};

/**
 * ModelToggles component displays all registered models with toggle
 * switches to show/hide them in the HUD input bar model selector.
 */
export function ModelToggles() {
  const [models, setModels] = useState<ModelEntry[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadModels = async () => {
      try {
        const result = await invoke<ModelEntry[]>("get_models");
        setModels(result);
      } catch (error) {
        console.error("[ModelToggles] Failed to load models:", error);
      } finally {
        setLoading(false);
      }
    };
    void loadModels();
  }, []);

  const handleToggle = useCallback(
    async (modelKey: string, currentEnabled: boolean) => {
      const newEnabled = !currentEnabled;
      try {
        await invoke("toggle_model", { modelKey, enabled: newEnabled });
        setModels((prev) =>
          prev.map((m) =>
            m.model === modelKey ? { ...m, is_enabled: newEnabled } : m,
          ),
        );
      } catch (error) {
        console.error(`[ModelToggles] Failed to toggle model ${modelKey}:`, error);
        toast.error("Failed to toggle model");
      }
    },
    [],
  );

  if (loading) {
    return (
      <div className="flex items-center justify-center p-8 text-sm text-muted-foreground">
        Loading models...
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {models.map((model) => {
        const IconComponent = ICON_MAP[model.icon] || Shield;
        return (
          <div
            key={model.model}
            className="flex flex-row items-center justify-between p-4"
          >
            <div className="flex items-center gap-3">
              <div
                className={`flex h-8 w-8 items-center justify-center rounded-full ${model.icon_bg}`}
              >
                <IconComponent className={`h-4 w-4 ${model.icon_color}`} />
              </div>
              <div className="flex flex-col">
                <p className="font-semibold text-sm">{model.display_name}</p>
                <p className="text-xs text-muted-foreground">
                  {model.short_description}
                </p>
              </div>
            </div>
            <Switch
              checked={model.is_enabled}
              onCheckedChange={() =>
                void handleToggle(model.model, model.is_enabled)
              }
            />
          </div>
        );
      })}
    </div>
  );
}
