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
import type { CloudModelUsage, ModelEntry } from "@/types/models";
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

function RemainingBadge({ usage }: { usage?: CloudModelUsage }) {
  if (!usage || !usage.is_available) return null;
  const { remaining, daily_limit } = usage;
  // Unlimited access (-1) or no daily limit — nothing to show
  if (remaining === -1 || daily_limit <= 0) return null;

  return (
    <span className={`text-xs ${remaining > 0 ? 'text-muted-foreground' : 'text-destructive font-medium'}`}>
      {remaining > 0 ? `${remaining}/${daily_limit} uses left today` : '0 uses left today'}
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
  const { enabledModels, cloudUsage } = useModelAccess();

  const handleChange = (v: string) => {
    const model = enabledModels.find((m) => m.id.toString() === v);

    if (model?.is_cloud) {
      const usage = cloudUsage[model.model];
      // Block if model is not available for this tier
      if (usage && !usage.is_available) return;
      // Block if at daily limit (remaining === -1 means unlimited)
      if (usage && usage.is_available && usage.remaining !== -1 && usage.remaining <= 0) return;
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
            const usage = cloudUsage[model.model];
            const isUnlimited = usage?.remaining === -1;
            const isUnavailable = model.is_cloud && usage?.is_available === false;
            const isAtLimit = model.is_cloud && !isUnavailable && !isUnlimited && usage?.is_available && usage.remaining <= 0;
            // Only use Radix disabled for at-limit models (blocks all interaction).
            // Unavailable premium models stay interactive so the Upgrade button is clickable —
            // handleChange() prevents the actual selection.
            const isRadixDisabled = !!isAtLimit;
            const isVisuallyDisabled = isUnavailable || !!isAtLimit;

            return (
              <div key={model.id}>
                {index > 0 && <SelectSeparator />}
                <SelectItem
                  value={model.id.toString()}
                  className={`py-4 px-4 cursor-pointer h-auto min-h-[4rem] ${isVisuallyDisabled ? 'opacity-50' : ''}`}
                  disabled={isRadixDisabled}
                >
                  <div className="flex items-center justify-between w-full">
                    <div className="flex items-center gap-3">
                      <ProviderIcon model={model} />
                      <div className="flex flex-col items-start">
                        <span className="font-medium">{model.display_name}</span>
                        <span className="text-xs text-muted-foreground text-left">
                          {model.description}
                        </span>
                        {model.is_cloud && usage?.is_available && (
                          <RemainingBadge usage={usage} />
                        )}
                      </div>
                    </div>
                    {model.is_premium && isUnavailable && (
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
