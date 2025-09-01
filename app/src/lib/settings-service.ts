"use client";

import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import { HudDimensions, HudSizeOption, ModelSelection, UserSettings } from "@/types/settings";

export class SettingsService {
  // Simple in-memory cache on the frontend
  private static cache: UserSettings | null = null;
  private static eventsHooked = false;

  private static async ensureEventsHooked() {
    if (this.eventsHooked) return;
    try {
      await listen("settings_changed", () => {
        // Invalidate local cache when another window updates settings
        this.cache = null;
      });
      this.eventsHooked = true;
    } catch (e) {
      console.warn("Failed to listen for settings_changed:", e);
    }
  }

  /**
   * Load user settings
   */
  static async loadSettings(): Promise<UserSettings> {
    try {
  await this.ensureEventsHooked();
      if (this.cache) return this.cache;
      const settings = await invoke<UserSettings>("load_user_settings");
      this.cache = settings;
      return this.cache;
    } catch (error) {
      console.error("Failed to load user settings:", error);
      // Return defaults on error
      const defaults: UserSettings = {
        hud_size: "Normal",
        model_selection: "Local",
      };
      // Cache defaults to keep behavior consistent within this session
      this.cache = defaults;
      return defaults;
    }
  }

  /**
   * Save user settings
   */
  static async saveSettings(settings: UserSettings): Promise<void> {
    try {
      await this.ensureEventsHooked();
      // Optimistically update cache
      this.cache = settings;
      await invoke("save_user_settings", { settings });
    } catch (error) {
      console.error("Failed to save user settings:", error);
      throw error;
    }

    // Emit a settings changed event
    await emit('settings_changed');
  }

  /**
   * Get current HUD size setting
   */
  static async getHudSize(): Promise<HudSizeOption> {
    try {
      const settings = await this.loadSettings();
      return settings.hud_size;
    } catch (error) {
      console.error("Failed to get HUD size setting:", error);
      return "Normal";
    }
  }

  /**
   * Set HUD size setting
   */
  static async setHudSize(size: HudSizeOption, isExpanded?: boolean): Promise<void> {
    try {
      // Get current settings
      const settings = await this.loadSettings();
      
      // Update the HUD size
      settings.hud_size = size;
      
      // Save the updated settings
      await this.saveSettings(settings);

      // Refresh the HUD window size immediately if we know the expanded state
      if (typeof isExpanded === 'boolean') {
        try {
          await invoke("refresh_hud_window_size", { 
            label: "floating-hud", 
            isExpanded: isExpanded 
          });
        } catch (refreshError) {
          console.warn("Failed to refresh HUD window size:", refreshError);
          // Not critical - the window will get the right size on next expand/collapse
        }
      }
    } catch (error) {
      console.error("Failed to set HUD size setting:", error);
      throw error;
    }
  }

  /**
   * Get current HUD dimensions
   */
  static async getHudDimensions(): Promise<HudDimensions> {
    try {
      const settings = await this.loadSettings();
      return this.hudSizeOptionToDimensions(settings.hud_size);
    } catch (error) {
      console.error("Failed to get HUD dimensions:", error);
      // Return defaults
      return {
        width: 500,
        collapsed_height: 60,
        expanded_height: 350,
      };
    }
  }

  /**
   * Convert HUD size option to dimensions (moved from backend)
   */
  private static hudSizeOptionToDimensions(option: HudSizeOption): HudDimensions {
    switch (option) {
      case "Small":
        return {
          width: 400,
          collapsed_height: 50,
          expanded_height: 250,
        };
      case "Large":
        return {
          width: 600,
          collapsed_height: 70,
          expanded_height: 450,
        };
      default: // Normal
        return {
          width: 500,
          collapsed_height: 60,
          expanded_height: 350,
        };
    }
  }

  /**
   * Get current model selection setting
   */
  static async getModelSelection(): Promise<ModelSelection> {
    try {
      const settings = await this.loadSettings();
      return settings.model_selection;
    } catch (error) {
      console.error("Failed to get model selection setting:", error);
      return "Local";
    }
  }

  /**
   * Set model selection setting
   */
  static async setModelSelection(selection: ModelSelection): Promise<void> {
    try {
      // Get current settings
      const settings = await this.loadSettings();

      // Update the model selection
      settings.model_selection = selection;

      // Save the updated settings
      await this.saveSettings(settings);
    } catch (error) {
      console.error("Failed to set model selection setting:", error);
      throw error;
    }
  }

  /**
   * Refresh settings cache (clear backend cache)
   */
  static async refreshCache(): Promise<void> {
    try {
      // Clear  cache
      this.cache = null;
    } catch (error) {
      console.error("Failed to refresh settings cache:", error);
    }
  }
}
