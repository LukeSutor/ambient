"use client";

import {
  DangerZone,
  ModelSelector,
  SettingsSection,
  SkillToggles,
} from "@/components/secondary/settings";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useSettings } from "@/lib/settings";
import type { GpuDevice } from "@/types/llm";
import type { HudSizeOption, ModelSelection } from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { toast } from "sonner";

const HUD_SIZE_OPTIONS: HudSizeOption[] = ["Small", "Normal", "Large"];

interface SettingRowProps {
  title: string;
  description: string;
  children: React.ReactNode;
}

function SettingRow({ title, description, children }: SettingRowProps) {
  return (
    <div className="flex flex-row items-center justify-between p-4">
      <div className="flex flex-col">
        <p className="font-semibold text-sm">{title}</p>
        <p className="text-sm text-gray-600">{description}</p>
      </div>
      {children}
    </div>
  );
}

export default function Settings() {
  const {
    settings,
    isLoading,
    setHudSize,
    setShowFullThoughtTraces,
    setModelSelection,
    setGpuAcceleration,
  } = useSettings();

  const [gpuDevices, setGpuDevices] = useState<GpuDevice[]>([]);
  const [gpuDetectionDone, setGpuDetectionDone] = useState(false);

  // Detect GPU devices on mount
  useEffect(() => {
    invoke<GpuDevice[]>("detect_gpu_devices")
      .then((devices) => {
        setGpuDevices(devices);
        setGpuDetectionDone(true);
      })
      .catch((error) => {
        console.warn("[Settings] GPU detection failed:", error);
        setGpuDetectionDone(true);
      });
  }, []);

  const hasGpu = gpuDevices.length > 0;

  const hudSize = settings?.hud_size ?? "Normal";
  const modelSelection = settings?.model_selection ?? "Local";

  const handleHudSizeChange = async (value: string) => {
    const newSize = value as HudSizeOption;
    try {
      await setHudSize(newSize);
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

  const handleGpuAccelerationChange = async (enabled: boolean) => {
    try {
      await setGpuAcceleration(enabled);
      toast.success(
        enabled
          ? "GPU acceleration enabled. Restart the app to apply."
          : "GPU acceleration disabled. Restart the app to apply.",
      );
    } catch (error) {
      console.error("Failed to save GPU acceleration setting:", error);
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
        <SettingRow
          title="GPU Acceleration"
          description={
            !gpuDetectionDone
              ? "Detecting GPU..."
              : hasGpu
                ? `Offload model to GPU via Vulkan (${gpuDevices[0].name})`
                : "No compatible GPU detected"
          }
        >
          <Switch
            checked={hasGpu && (settings?.gpu_acceleration ?? false)}
            onCheckedChange={(checked) => {
              void handleGpuAccelerationChange(checked);
            }}
            disabled={isLoading || !gpuDetectionDone || !hasGpu}
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
        <SettingRow
          title="Show Full Thought Traces"
          description="Toggle whether to show the full thought traces in the HUD"
        >
          <Switch
            checked={settings?.show_full_thought_traces ?? false}
            onCheckedChange={(checked) => {
              void setShowFullThoughtTraces(checked);
            }}
          />
        </SettingRow>
      </SettingsSection>

      {/* Tools Settings */}
      <SettingsSection title="Tools">
        <SkillToggles />
      </SettingsSection>

      {/* Danger zone */}
      <SettingsSection title="Danger Zone" variant="danger">
        <DangerZone onReset={() => void handleReset()} />
      </SettingsSection>
    </div>
  );
}
