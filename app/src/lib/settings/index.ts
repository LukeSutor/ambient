/**
 * Settings Management Library
 * Single source of truth for user settings with caching
 */

// Provider for shared settings state
export { SettingsProvider } from './SettingsProvider';

// Main hook for settings functionality
export { useSettings } from './useSettings';

// Re-export types for convenience
export type { UserSettings, HudSizeOption, ModelSelection, HudDimensions } from '@/types/settings';
