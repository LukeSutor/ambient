import { invoke } from '@tauri-apps/api/core';

/**
 * Resize the HUD window to specified dimensions.
 * @param width 
 * @param height 
 */

export async function resize_hud(width: number, height: number): Promise<void> {
  try {
    await invoke('resize_hud', { width, height });
  } catch (error) {
    console.error('[WindowsAPI] Failed to resize HUD:', error);
    throw new Error('Failed to resize HUD');
  }
}