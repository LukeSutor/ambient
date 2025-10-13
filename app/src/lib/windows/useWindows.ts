'use client';

import { useEffect, useRef, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();

    // ============================================================
    // Operations
    // ============================================================
    const setLogin = useCallback(() => {
        dispatch({ type: 'SET_LOGIN' });
    }, [dispatch]);

    const minimizeChat = useCallback(async () => {
        const dimensions = getHudDimensions();
        try {
            await invoke('resize_hud', { width: dimensions.chat_width, height: dimensions.input_bar_height });
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
        } catch (error) {
            console.error('[useWindows] Failed to minimize chat HUD:', error);
        }
    }, [dispatch, getHudDimensions]);

    const expandChat = useCallback(async (scrollHeight: number) => {
        const dimensions = getHudDimensions();
        try {
            await invoke('resize_hud', { width: dimensions.chat_width, height: scrollHeight });
            dispatch({ type: 'SET_EXPANDED_CHAT' });
        } catch (error) {
            console.error('[useWindows] Failed to expand chat HUD:', error);
        }
    }, [dispatch, getHudDimensions]);

    // ============================================================
    // Return API
    // ============================================================
    return {
        isLogin: state.isLogin,
        isExpanded: state.isExpanded,
        setLogin,
        minimizeChat,
        expandChat
    };
}