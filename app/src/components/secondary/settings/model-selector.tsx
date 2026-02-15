"use client";

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
import type { ModelEntry } from "@/types/models";
import { Zap } from "lucide-react";

/** Resolve a provider image path. Local uses `/logo.png`, everything else
 *  uses `/providers/{provider}.webp` with a fallback to `unknown.webp`. */
function providerImageSrc(model: ModelEntry): string {
  if (model.provider === "local") return "/logo.png";
  return `/providers/${model.provider}.webp`;
}

interface ModelSelectorProps {
  /** The currently selected model id (as string, e.g. "1"). */
  value: string;
  onChange: (value: string) => void;
  disabled?: boolean;
}

function ProviderIcon({ model }: { model: ModelEntry }) {
  return (
    <div className="flex h-7 w-7 items-center justify-center rounded-full bg-muted">
      <img
        src={providerImageSrc(model)}
        alt={model.provider}
        className="h-4 w-4 object-contain"
        onError={(e) => {
          (e.target as HTMLImageElement).src = "/providers/unknown.webp";
        }}
      />
    </div>
  );
}

function CreditCostBadge({ cost }: { cost?: number }) {
  if (cost === undefined) return null;
  return (
    <span className="text-xs text-muted-foreground">
      {cost} credit{cost !== 1 ? "s" : ""} per use
    </span>
  );
}

function SelectedModelDisplay({ value, models }: { value: string; models: ModelEntry[] }) {
  const model = models.find((m) => m.id.toString() === value);
  if (!model) {
    // Show a neutral placeholder while models are loading
    return <span className="font-medium text-muted-foreground">Loading...</span>;
  }

  return (
    <div className="flex items-center gap-3">
      <ProviderIcon model={model} />
      <span className="font-medium">{model.display_name}</span>
    </div>
  );
}

export function ModelSelector({
  value,
  onChange,
  disabled,
}: ModelSelectorProps) {
  const { enabledModels, canAffordModel, modelCosts } = useModelAccess();

  const handleChange = (v: string) => {
    const model = enabledModels.find((m) => m.id.toString() === v);

    // Block if user can't afford this model
    if (model?.is_cloud && model.is_internal && !canAffordModel(model.model)) return;

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
          <SelectedModelDisplay value={value} models={enabledModels} />
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="w-96" align="end">
        <SelectGroup>
          <SelectLabel className="text-xs font-medium text-muted-foreground px-2 py-1.5 flex items-center gap-2">
            <Zap className="h-3 w-3" />
            Available Models
          </SelectLabel>

          {enabledModels.map((model, index) => {
            const cost = model.is_cloud && model.is_internal ? modelCosts[model.model] : undefined;
            const isAffordable = canAffordModel(model.model);
            const isUnaffordable = model.is_cloud && model.is_internal && !isAffordable;
            const isVisuallyDisabled = isUnaffordable;

            return (
              <div key={model.id}>
                {index > 0 && <SelectSeparator />}
                <SelectItem
                  value={model.id.toString()}
                  className={`py-4 px-4 cursor-pointer h-auto min-h-[4rem] ${isVisuallyDisabled ? 'opacity-50' : ''}`}
                  disabled={isUnaffordable}
                >
                  <div className="flex items-center justify-between w-full">
                    <div className="flex items-center gap-3">
                      <ProviderIcon model={model} />
                      <div className="flex flex-col items-start">
                        <span className="font-medium">{model.display_name}</span>
                        <span className="text-xs text-muted-foreground text-left">
                          {model.description}
                        </span>
                        {model.is_cloud && model.is_internal && (
                          <CreditCostBadge cost={cost} />
                        )}
                        {isUnaffordable && (
                          <span className="text-xs text-destructive font-medium">Not enough credits</span>
                        )}
                      </div>
                    </div>
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
