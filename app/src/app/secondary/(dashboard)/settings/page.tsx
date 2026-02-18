"use client";

import {
  DangerZone,
  ModelSelector,
  ModelVisibilityPopover,
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
import { Input } from "@/components/ui/input";
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
  titleAction?: React.ReactNode;
}

function SettingRow({
  title,
  description,
  children,
  titleAction,
}: SettingRowProps) {
  return (
    <div className="flex flex-row items-center justify-between p-4">
      <div className="flex flex-col">
        <div className="flex items-center gap-2">
          <p className="font-semibold text-sm">{title}</p>
          {titleAction}
        </div>
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
    setScreenPollInterval,
  } = useSettings();

  const [pollIntervalInput, setPollIntervalInput] = useState<string>("");

  const [gpuDevices, setGpuDevices] = useState<GpuDevice[]>([]);
  const [gpuDetectionDone, setGpuDetectionDone] = useState(false);

  // Detect GPU devices on mount
  useEffect(() => {
    invoke<GpuDevice[]>("detect_gpu_devices")
      .then((devices) => {
        setGpuDevices(devices);
        setGpuDetectionDone(true);
      })
      .catch((error: unknown) => {
        console.warn("[Settings] GPU detection failed:", error);
        setGpuDetectionDone(true);
      });
  }, []);

  const hasGpu = gpuDevices.length > 0;

  // Sync poll interval input with settings
  useEffect(() => {
    setPollIntervalInput(String(settings?.screen_poll_interval_secs ?? 30));
  }, [settings?.screen_poll_interval_secs]);

  const handlePollIntervalBlur = async () => {
    const n = Number(pollIntervalInput);
    if (Number.isNaN(n) || n < 5 || n > 300) {
      toast.error("Poll interval must be between 5 and 300 seconds");
      setPollIntervalInput(String(settings?.screen_poll_interval_secs ?? 30));
      return;
    }
    try {
      await setScreenPollInterval(n);
      toast.success("Screen monitor interval updated");
    } catch (error) {
      console.error("Failed to save poll interval:", error);
      toast.error("Failed to save setting");
    }
  };

  const hudSize = settings?.hud_size ?? "Normal";
  const modelSelection = settings?.model_selection ?? "1";

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
      toast.success("Model selection updated");
    } catch (error) {
      console.error("Failed to save model selection setting:", error);
      toast.error("Failed to save setting");
    }
  };

  const handleGpuAccelerationChange = async (enabled: boolean) => {
    try {
      await setGpuAcceleration(enabled);
      toast.info("Restarting local model server...");

      try {
        await invoke("restart_llama_server");
        toast.success(
          enabled
            ? "GPU acceleration enabled. Server restarted."
            : "GPU acceleration disabled. Server restarted.",
        );
      } catch (restartError) {
        console.error("Failed to restart server:", restartError);
        toast.error(
          "Setting saved but server restart failed. Please restart the app.",
        );
      }
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
          titleAction={<ModelVisibilityPopover />}
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
                ? `Offload model to ${gpuDevices[0].name}`
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
          description="Toggle whether to show the full thought traces in conversations"
        >
          <Switch
            checked={settings?.show_full_thought_traces ?? false}
            onCheckedChange={(checked) => {
              void setShowFullThoughtTraces(checked);
            }}
          />
        </SettingRow>
      </SettingsSection>

      {/* Automations Settings */}
      <SettingsSection title="Automations">
        <SettingRow
          title="Screen Monitor Interval"
          description="How often to check screen content for automation triggers (5-300 seconds)"
        >
          <div className="flex items-center gap-2">
            <Input
              type="number"
              min={5}
              max={300}
              step={5}
              className="w-24 text-right"
              value={pollIntervalInput}
              onChange={(e) => setPollIntervalInput(e.target.value)}
              onBlur={() => void handlePollIntervalBlur()}
              disabled={isLoading}
            />
            <span className="text-sm text-muted-foreground">sec</span>
          </div>
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
