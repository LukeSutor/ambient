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
      {remaining}/{daily_limit} left today
    </span>
  );
}

function SelectedModelDisplay({ value, models }: { value: string; models: ModelEntry[] }) {
  const model = models.find((m) => m.model === value);
  if (!model) {
    // Fallback while loading
    return <span className="font-medium">{value}</span>;
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

  const handleChange = (v: string) => {
    const model = models.find((m) => m.model === v);

    // Prevent selecting premium models without upgrade
    if (model?.is_premium) return;

    // Prevent selecting cloud models with 0 remaining uses
    // Cloud usage is keyed by api_model_name (e.g. "fast", "pro")
    if (model?.is_cloud && !model.is_premium) {
      const usage = cloudUsage[model.api_model_name];
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
            // Cloud usage is keyed by api_model_name (e.g. "fast", "pro")
            const usage = cloudUsage[model.api_model_name];
            const isAtLimit = model.is_cloud && !model.is_premium && usage && usage.remaining <= 0;
            const isDisabled = model.is_premium || !!isAtLimit;

            return (
              <div key={model.model}>
                {index > 0 && <SelectSeparator />}
                <SelectItem
                  value={model.model}
                  className={`py-4 px-4 cursor-pointer h-auto min-h-[4rem] ${isDisabled ? 'opacity-50' : ''}`}
                  disabled={isDisabled}
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
