'use client';

import { useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useSettingsContext } from './SettingsProvider';
import { UserSettings, HudSizeOption, ModelSelection, HudDimensions } from '@/types/settings';

/**
 * Convert HUD size option to dimensions
 */
function hudSizeOptionToDimensions(option: HudSizeOption): HudDimensions {
  switch (option) {
    case "Small":
      return {
        default_width: 400,
        default_height: 50,
        chat_width: 400,
        input_bar_height: 100,
        chat_max_height: 250,
        login_width: 450,
        login_height: 600,
      };
    case "Large":
      return {
        default_width: 600,
        default_height: 70,
        chat_width: 600,
        input_bar_height: 120,
        chat_max_height: 450,
        login_width: 450,
        login_height: 600,
      };
    default: // Normal
      return {
        default_width: 500,
        default_height: 60,
        chat_width: 500,
        input_bar_height: 120,
        chat_max_height: 350,
        login_width: 450,
        login_height: 600,
      };
  }
}

/**
 * Main settings hook - provides all settings functionality
 * 
 * @returns Settings state and operations
 */
export function useSettings() {
  const { state, dispatch } = useSettingsContext();

  // ============================================================
  // Event Listener Setup
  // ============================================================

  useEffect(() => {
    let isMounted = true;

    const setupEvents = async () => {
      try {
        // Listen for settings changes from other windows/sources
        const unlisten = await listen('settings_changed', async () => {
          console.log('[useSettings] Settings changed event received, invalidating cache');
          
          // Invalidate cache and reload
          dispatch({ type: 'INVALIDATE_CACHE' });
          
          if (isMounted) {
            try {
              const settings = await invoke<UserSettings>('load_user_settings');
              dispatch({ type: 'SET_SETTINGS', payload: settings });
            } catch (error) {
              console.error('[useSettings] Failed to reload settings:', error);
            }
          }
        });

        return unlisten;
      } catch (error) {
        console.error('[useSettings] Failed to setup event listener:', error);
        return null;
      }
    };

    let cleanup: UnlistenFn | null = null;
    setupEvents().then((fn) => {
      if (isMounted) {
        cleanup = fn;
      } else if (fn) {
        fn();
      }
    });

    return () => {
      isMounted = false;
      if (cleanup) {
        cleanup();
      }
    };
  }, [dispatch]);

  // ============================================================
  // Initialization Effect
  // ============================================================

  useEffect(() => {
    // Check shared initialization ref to prevent multiple initializations
    if (state.initializationRef.current || state.settings) {
      return;
    }
    
    state.initializationRef.current = true;

    const initialize = async () => {
      console.log('[useSettings] Initializing settings...');
      
      try {
        dispatch({ type: 'SET_LOADING', payload: true });
        const settings = await invoke<UserSettings>('load_user_settings');
        dispatch({ type: 'SET_SETTINGS', payload: settings });
      } catch (error) {
        console.error('[useSettings] Failed to load settings:', error);
        
        // Set defaults on error
        const defaults: UserSettings = {
          hud_size: 'Normal',
          model_selection: 'Local',
        };
        dispatch({ type: 'SET_SETTINGS', payload: defaults });
      }
    };

    initialize();
  }, [state.initializationRef, state.settings, dispatch]);

  // ============================================================
  // Operations
  // ============================================================

  /**
   * Loads settings from backend (bypasses cache)
   */
  const loadSettings = useCallback(async (): Promise<UserSettings> => {
    try {
      dispatch({ type: 'SET_LOADING', payload: true });
      const settings = await invoke<UserSettings>('load_user_settings');
      dispatch({ type: 'SET_SETTINGS', payload: settings });
      return settings;
    } catch (error) {
      console.error('[useSettings] Failed to load settings:', error);
      throw error;
    }
  }, [dispatch]);

  /**
   * Saves settings to backend and updates cache
   */
  const saveSettings = useCallback(async (settings: UserSettings): Promise<void> => {
    try {
      // Optimistically update cache
      dispatch({ type: 'SET_SETTINGS', payload: settings });
      
      // Save to backend
      await invoke('save_user_settings', { settings });
      
      // Emit settings changed event for other windows
      await invoke('emit_settings_changed');
      
      console.log('[useSettings] Settings saved successfully');
    } catch (error) {
      console.error('[useSettings] Failed to save settings:', error);
      
      // Reload from backend on error to ensure consistency
      await loadSettings();
      throw error;
    }
  }, [dispatch, loadSettings]);

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
  const setHudSize = useCallback(async (size: HudSizeOption, isExpanded?: boolean): Promise<void> => {
    try {
      if (!state.settings) {
        throw new Error('Settings not loaded');
      }

      // Optimistically update
      dispatch({ type: 'UPDATE_HUD_SIZE', payload: size });

      // Save to backend
      const updatedSettings = {
        ...state.settings,
        hud_size: size,
      };
      await invoke('save_user_settings', { settings: updatedSettings });

      // Refresh HUD window size
        try {
            await invoke('refresh_hud_window_size', { 
            label: 'main', 
            isExpanded 
            });
        } catch (refreshError) {
            console.warn('[useSettings] Failed to refresh HUD window:', refreshError);
        }

      // Emit settings changed event
      await invoke('emit_settings_changed');
      
      console.log('[useSettings] HUD size updated to:', size);
    } catch (error) {
      console.error('[useSettings] Failed to set HUD size:', error);
      
      // Reload settings on error
      await loadSettings();
      throw error;
    }
  }, [state.settings, dispatch, loadSettings]);

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
  const setModelSelection = useCallback(async (selection: ModelSelection): Promise<void> => {
    try {
      if (!state.settings) {
        throw new Error('Settings not loaded');
      }

      // Optimistically update
      dispatch({ type: 'UPDATE_MODEL_SELECTION', payload: selection });

      // Save to backend
      const updatedSettings = {
        ...state.settings,
        model_selection: selection,
      };
      await invoke('save_user_settings', { settings: updatedSettings });

      // Emit settings changed event
      await invoke('emit_settings_changed');
      
      console.log('[useSettings] Model selection updated to:', selection);
    } catch (error) {
      console.error('[useSettings] Failed to set model selection:', error);
      
      // Reload settings on error
      await loadSettings();
      throw error;
    }
  }, [state.settings, dispatch, loadSettings]);

  /**
   * Invalidates the cache and reloads from backend
   */
  const refreshCache = useCallback(async (): Promise<void> => {
    console.log('[useSettings] Refreshing cache...');
    dispatch({ type: 'INVALIDATE_CACHE' });
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
