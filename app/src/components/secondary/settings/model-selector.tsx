"use client";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectSeparator,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useModelAccess } from "@/lib/model-access";
import type { ModelEntry, CloudModelUsage } from "@/types/models";
import { Crown, Shield, Zap } from "lucide-react";
import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

const ICON_MAP: Record<string, React.ElementType> = {
  shield: Shield,
  zap: Zap,
  crown: Crown,
};

interface ModelSelectorProps {
  /** The currently selected model key (e.g. "qwen3vl-2b"). */
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

function ModelIcon({ model }: { model: ModelEntry }) {
  const IconComponent = ICON_MAP[model.icon] || Shield;
  return (
    <div
      className={`flex items-center justify-center rounded-full ${model.icon_bg}`}
    >
      <IconComponent className={`h-4 w-4 m-1.5 ${model.icon_color}`} />
    </div>
  );
}

function RemainingBadge({ usage }: { usage?: CloudModelUsage }) {
  if (!usage) return null;
  const { remaining, daily_limit } = usage;
  if (daily_limit <= 0) return null;

  return (
    <span className={`text-xs ${remaining > 0 ? 'text-muted-foreground' : 'text-destructive font-medium'}`}>
      {remaining > 0 ? `${remaining}/${daily_limit} uses left today` : '0 uses left today'}
    </span>
  );
}

function SelectedModelDisplay({ value, models }: { value: string; models: ModelEntry[] }) {
  const model = models.find((m) => m.model === value);
  if (!model) {
    // Show a neutral placeholder while models are loading
    return <span className="font-medium text-muted-foreground">Loading...</span>;
  }

  return (
    <div className="flex items-center gap-3">
      <div
        className={`flex h-6 w-6 items-center justify-center rounded-full ${model.icon_bg}`}
      >
        {(() => {
          const IconComponent = ICON_MAP[model.icon] || Shield;
          return <IconComponent className={`h-4 w-4 ${model.icon_color}`} />;
        })()}
      </div>
      <span className="font-medium">{model.display_name}</span>
    </div>
  );
}

export function ModelSelector({
  value,
  onChange,
  disabled,
}: ModelSelectorProps) {
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

  const handleChange = (v: string) => {
    const model = models.find((m) => m.model === v);

    // Prevent selecting premium models without upgrade
    if (model?.is_premium) return;

    // Prevent selecting cloud models with 0 remaining uses
    if (model?.is_cloud && !model.is_premium) {
      const usage = cloudUsage[model.model];
      if (usage && usage.remaining <= 0) return;
    }

    onChange(v);
  };

  return (
    <Select
      value={value}
      onValueChange={handleChange}
      disabled={disabled}
    >
      <SelectTrigger>
        <SelectValue placeholder="Select model">
          <SelectedModelDisplay value={value} models={models} />
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="w-96" align="end">
        <SelectGroup>
          <SelectLabel className="text-xs font-medium text-muted-foreground px-2 py-1.5 flex items-center gap-2">
            <Zap className="h-3 w-3" />
            Available Models
          </SelectLabel>

          {models.map((model, index) => {
            const usage = cloudUsage[model.model];
            const isAtLimit = model.is_cloud && !model.is_premium && usage && usage.remaining <= 0;
            // Only use Radix disabled for at-limit models (blocks all interaction).
            // Premium models stay interactive so the Upgrade button is clickable —
            // handleChange() prevents the actual selection.
            const isRadixDisabled = !!isAtLimit;
            const isVisuallyDisabled = model.is_premium || !!isAtLimit;

            return (
              <div key={model.model}>
                {index > 0 && <SelectSeparator />}
                <SelectItem
                  value={model.model}
                  className={`py-4 px-4 cursor-pointer h-auto min-h-[4rem] ${isVisuallyDisabled ? 'opacity-50' : ''}`}
                  disabled={isRadixDisabled}
                >
                  <div className="flex items-center justify-between w-full">
                    <div className="flex items-center gap-3">
                      <ModelIcon model={model} />
                      <div className="flex flex-col items-start">
                        <div className="flex items-center gap-2">
                          <span className="font-medium">{model.display_name}</span>
                          <Badge
                            variant={model.badge_variant as "outline" | "default"}
                            className={`text-xs ${model.badge_class}`}
                          >
                            {model.badge_label}
                          </Badge>
                        </div>
                        <span className="text-xs text-muted-foreground text-left">
                          {model.description}
                        </span>
                        {model.is_cloud && !model.is_premium && (
                          <RemainingBadge usage={usage} />
                        )}
                      </div>
                    </div>
                    {model.is_premium && (
                      <Button
                        variant="outline"
                        size="sm"
                        className="h-6 mr-4 text-xs px-2 bg-gradient-to-r from-purple-50 to-pink-50 border-purple-200 hover:from-purple-100 hover:to-pink-100"
                        onPointerDown={(e) => e.stopPropagation()}
                        onClick={(e) => {
                          e.stopPropagation();
                          window.location.href = "/secondary/upgrade";
                        }}
                      >
                        Upgrade
                      </Button>
                    )}
                  </div>
                </SelectItem>
              </div>
            );
          })}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}
