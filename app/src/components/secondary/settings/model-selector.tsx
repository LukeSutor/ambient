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
import type { ModelSelection } from "@/types/settings";
import { Crown, Shield, Zap } from "lucide-react";

interface ModelConfig {
  value: ModelSelection;
  name: string;
  description: string;
  icon: React.ReactNode;
  iconBgClass: string;
  badge: {
    label: string;
    variant: "outline" | "default";
    className?: string;
  };
  upgradeButton?: boolean;
}

const MODEL_CONFIGS: ModelConfig[] = [
  {
    value: "Local",
    name: "Local",
    description: "Ultimate privacy. Runs on your device. No internet required.",
    icon: <Shield className="h-4 w-4 m-1.5 text-green-600" />,
    iconBgClass: "bg-green-100",
    badge: { label: "Private", variant: "outline" },
  },
  {
    value: "Fast",
    name: "Gemini 3 Flash",
    description: "More powerful. Google's fast model with advanced capabilities.",
    icon: <Zap className="h-4 w-4 m-1.5 text-blue-600" />,
    iconBgClass: "bg-blue-100",
    badge: { label: "Enhanced", variant: "outline" },
  },
  {
    value: "Pro",
    name: "Gemini 3 Pro",
    description: "The latest and most advanced model from Google.",
    icon: <Crown className="h-4 w-4 m-1.5 text-white" />,
    iconBgClass: "bg-gradient-to-r from-purple-500 to-pink-500",
    badge: {
      label: "Premium",
      variant: "default",
      className: "bg-gradient-to-r from-purple-500 to-pink-500 border-none",
    },
    upgradeButton: true,
  },
];

interface ModelSelectorProps {
  value: ModelSelection;
  onChange: (value: ModelSelection) => void;
  disabled?: boolean;
}

function ModelIcon({ config }: { config: ModelConfig }) {
  return (
    <div
      className={`flex items-center justify-center rounded-full ${config.iconBgClass}`}
    >
      {config.icon}
    </div>
  );
}

function SelectedModelDisplay({ value }: { value: ModelSelection }) {
  const config = MODEL_CONFIGS.find((m) => m.value === value);
  if (!config) return null;

  return (
    <div className="flex items-center gap-3">
      <div
        className={`flex h-6 w-6 items-center justify-center rounded-full ${config.iconBgClass}`}
      >
        {config.icon}
      </div>
      <span className="font-medium">{config.name}</span>
    </div>
  );
}

export function ModelSelector({
  value,
  onChange,
  disabled,
}: ModelSelectorProps) {
  return (
    <Select
      value={value}
      onValueChange={(v) => onChange(v as ModelSelection)}
      disabled={disabled}
    >
      <SelectTrigger>
        <SelectValue placeholder="Select model">
          <SelectedModelDisplay value={value} />
        </SelectValue>
      </SelectTrigger>
      <SelectContent className="w-96">
        <SelectGroup>
          <SelectLabel className="text-xs font-medium text-muted-foreground px-2 py-1.5 flex items-center gap-2">
            <Zap className="h-3 w-3" />
            Available Models
          </SelectLabel>

          {MODEL_CONFIGS.map((config, index) => (
            <div key={config.value}>
              {index > 0 && <SelectSeparator />}
              <SelectItem
                value={config.value}
                className="py-4 px-4 cursor-pointer h-auto min-h-[4rem]"
              >
                <div className="flex items-center justify-between w-full">
                  <div className="flex items-center gap-3">
                    <ModelIcon config={config} />
                    <div className="flex flex-col items-start">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{config.name}</span>
                        <Badge
                          variant={config.badge.variant}
                          className={`text-xs ${config.badge.className || ""}`}
                        >
                          {config.badge.label}
                        </Badge>
                      </div>
                      <span className="text-xs text-muted-foreground text-left">
                        {config.description}
                      </span>
                    </div>
                  </div>
                  {config.upgradeButton && (
                    <Button
                      variant="outline"
                      size="sm"
                      className="h-6 mr-4 text-xs px-2 bg-gradient-to-r from-purple-50 to-pink-50 border-purple-200 hover:from-purple-100 hover:to-pink-100"
                    >
                      Upgrade
                    </Button>
                  )}
                </div>
              </SelectItem>
            </div>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}
