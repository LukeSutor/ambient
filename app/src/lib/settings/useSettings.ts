"use client";

import type {
  HudDimensions,
  HudSizeOption,
  ModelSelection,
  UserSettings,
} from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import { useCallback } from "react";
import { useSettingsContext } from "./SettingsProvider";

/**
 * Convert HUD size option to dimensions
 */
function hudSizeOptionToDimensions(option: HudSizeOption): HudDimensions {
  switch (option) {
    case "Small":
      return {
        chat_width: 400,
        input_bar_height: 130,
        chat_max_height: 250,
        login_width: 450,
        login_height: 600,
      };
    case "Large":
      return {
        chat_width: 700,
        input_bar_height: 130,
        chat_max_height: 600,
        login_width: 450,
        login_height: 600,
      };
    default: // Normal
      return {
        chat_width: 600,
        input_bar_height: 130,
        chat_max_height: 450,
        login_width: 450,
        login_height: 600,
      };
  }
}

/**
 * Main settings hook - provides all settings functionality
 * @returns Settings state and operations
 */
export function useSettings() {
  const { state, dispatch } = useSettingsContext();

  // ============================================================
  // Operations
  // ============================================================

  /**
   * Loads settings from backend (bypasses cache)
   */
  const loadSettings = useCallback(async (): Promise<UserSettings> => {
    try {
      dispatch({ type: "SET_LOADING", payload: true });
      const settings = await invoke<UserSettings>("load_user_settings");
      dispatch({ type: "SET_SETTINGS", payload: settings });
      return settings;
    } catch (error) {
      console.error("[useSettings] Failed to load settings:", error);
      throw error;
    }
  }, [dispatch]);

  /**
   * Saves settings to backend and updates cache
   */
  const saveSettings = useCallback(
    async (settings: UserSettings): Promise<void> => {
      try {
        // Optimistically update cache
        dispatch({ type: "SET_SETTINGS", payload: settings });

        // Save to backend
        await invoke("save_user_settings", { settings });

        // Emit settings changed event for other windows
        await invoke("emit_settings_changed");
      } catch (error) {
        console.error("[useSettings] Failed to save settings:", error);

        // Reload from backend on error to ensure consistency
        await loadSettings();
        throw error;
      }
    },
    [dispatch, loadSettings],
  );

  /**
   * Gets current HUD size setting
   */
  const getHudSize = useCallback(async (): Promise<HudSizeOption> => {
    // Ensure settings are loaded before returning a value
    if (state.settings) {
      return state.settings.hud_size;
    }
    const settings = await loadSettings();
    return settings.hud_size;
  }, [state.settings, loadSettings]);

  /**
   * Sets HUD size setting
   */
  const setHudSize = useCallback(
    async (size: HudSizeOption): Promise<void> => {
      try {
        if (!state.settings) {
          throw new Error("Settings not loaded");
        }

        // Optimistically update
        dispatch({ type: "UPDATE_HUD_SIZE", payload: size });

        // Save to backend
        const updatedSettings = {
          ...state.settings,
          hud_size: size,
        };
        await invoke("save_user_settings", { settings: updatedSettings });

        // Refresh main window size
        try {
          await invoke("refresh_main_window_size");
        } catch (refreshError) {
          console.warn(
            "[useSettings] Failed to refresh main window:",
            refreshError,
          );
        }

        // Emit settings changed event
        await invoke("emit_settings_changed");
      } catch (error) {
        console.error("[useSettings] Failed to set HUD size:", error);

        // Reload settings on error
        await loadSettings();
        throw error;
      }
    },
    [state.settings, dispatch, loadSettings],
  );

  /**
   * Gets HUD dimensions based on current size setting
   */
  const getHudDimensions = useCallback(async (): Promise<HudDimensions> => {
    const size = await getHudSize();
    return hudSizeOptionToDimensions(size);
  }, [getHudSize]);

  /**
   * Gets current model selection setting
   */
  const getModelSelection = useCallback(async (): Promise<ModelSelection> => {
    // Ensure settings are loaded before returning a value
    if (state.settings) {
      return state.settings.model_selection;
    }
    const settings = await loadSettings();
    return settings.model_selection;
  }, [state.settings, loadSettings]);

  /**
   * Sets model selection setting
   */
  const setModelSelection = useCallback(
    async (selection: ModelSelection): Promise<void> => {
      try {
        if (!state.settings) {
          throw new Error("Settings not loaded");
        }

        // Optimistically update
        dispatch({ type: "UPDATE_MODEL_SELECTION", payload: selection });

        // Save to backend
        const updatedSettings = {
          ...state.settings,
          model_selection: selection,
        };
        await invoke("save_user_settings", { settings: updatedSettings });

        // Emit settings changed event
        await invoke("emit_settings_changed");
      } catch (error) {
        console.error("[useSettings] Failed to set model selection:", error);

        // Reload settings on error
        await loadSettings();
        throw error;
      }
    },
    [state.settings, dispatch, loadSettings],
  );

  /**
   * Invalidates the cache and reloads from backend
   */
  const refreshCache = useCallback(async (): Promise<void> => {
    dispatch({ type: "INVALIDATE_CACHE" });
    await loadSettings();
  }, [dispatch, loadSettings]);

  // ============================================================
  // Return API
  // ============================================================

  return {
    // State
    settings: state.settings,
    isLoading: state.isLoading,

    // Operations
    loadSettings,
    saveSettings,
    getHudSize,
    setHudSize,
    getHudDimensions,
    getModelSelection,
    setModelSelection,
    refreshCache,
  };
}
