"use client";

import {
  DangerZone,
  ModelSelector,
  SettingsSection,
} from "@/components/secondary/settings";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useSettings } from "@/lib/settings";
import type { HudSizeOption, ModelSelection } from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";

const HUD_SIZE_OPTIONS: HudSizeOption[] = ["Small", "Normal", "Large"];

interface SettingRowProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

function SettingRow({ title, description, children }: SettingRowProps) {
  return (
    <div className="flex flex-row justify-between p-4">
      <div className="flex flex-col">
        <p className="font-semibold text-sm">{title}</p>
        <p className="text-sm text-gray-600">{description}</p>
      </div>
      {children}
    </div>
  );
}

export default function Settings() {
  const { settings, isLoading, setHudSize, setModelSelection } = useSettings();

  const hudSize = settings?.hud_size ?? "Normal";
  const modelSelection = settings?.model_selection ?? "Local";

  const handleHudSizeChange = async (value: string) => {
    const newSize = value as HudSizeOption;
    try {
      await setHudSize(newSize, true);
      const displayName = newSize.charAt(0).toUpperCase() + newSize.slice(1);
      toast.success(`HUD size changed to ${displayName}`);
    } catch (error) {
      console.error("Failed to save HUD size setting:", error);
      toast.error("Failed to save setting");
    }
  };

  const handleModelSelectionChange = async (value: ModelSelection) => {
    try {
      await setModelSelection(value);
      const displayName = value.charAt(0).toUpperCase() + value.slice(1);
      toast.success(`Model selection changed to ${displayName}`);
    } catch (error) {
      console.error("Failed to save model selection setting:", error);
      toast.error("Failed to save setting");
    }
  };

  const handleReset = async () => {
    try {
      await invoke("reset_database");
      toast.success("Database reset successful");
    } catch (error) {
      console.error("Failed to reset database:", error);
      toast.error("Database reset not successful");
    }
  };

  return (
    <div className="relative flex flex-col items-center justify-center p-4 max-w-2xl w-full mx-auto">
      {/* Model Settings */}
      <SettingsSection title="Model Settings">
        <SettingRow
          title="Model Selection"
          description="Choose the model to use for processing"
        >
          <ModelSelector
            value={modelSelection}
            onChange={(v) => void handleModelSelectionChange(v)}
            disabled={isLoading}
          />
        </SettingRow>
      </SettingsSection>

      {/* Display Settings */}
      <SettingsSection title="Display Settings">
        <SettingRow
          title="Display Size"
          description="Choose the size of the chat display window"
        >
          <Select
            value={hudSize}
            onValueChange={(v) => void handleHudSizeChange(v)}
            disabled={isLoading}
          >
            <SelectTrigger className="w-32">
              <SelectValue placeholder="Select size" />
            </SelectTrigger>
            <SelectContent>
              {HUD_SIZE_OPTIONS.map((size) => (
                <SelectItem key={size} value={size}>
                  {size}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </SettingRow>
      </SettingsSection>

      {/* Danger zone */}
      <SettingsSection title="Danger Zone" variant="danger">
        <DangerZone onReset={() => void handleReset()} />
      </SettingsSection>
    </div>
  );
}
