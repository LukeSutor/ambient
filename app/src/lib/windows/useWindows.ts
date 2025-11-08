'use client';

import { useCallback, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';
import { usePathname } from 'next/navigation';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();
    const pathname = usePathname();

    // ============================================================
    // Effects
    // ============================================================
    useEffect(() => {
        // Set initial window size based on route
        (async () => {
            const dimensions = await getHudDimensions();
            if (pathname === '/hud/signin' || pathname === '/hud/signup' || pathname === '/hud/setup') {
                try {
                    await invoke('resize_hud', {
                        width: dimensions.login_width,
                        height: dimensions.login_height,
                    });
                } catch (error) {
                    console.error('[useWindows] Failed to resize for login/signup:', error);
                }
            } else {
                try {
                    await invoke('resize_hud', {
                        width: dimensions.chat_width,
                        height: dimensions.input_bar_height,
                    });
                } catch (error) {
                    console.error('[useWindows] Failed to resize for main HUD:', error);
                }
            }
        })();
    }, [pathname]);

    // ============================================================
    // Operations
    // ============================================================
    const setChatMinimized = useCallback((delay?: number) => {
        if (delay) {
            setTimeout(() => {
                dispatch({ type: 'SET_MINIMIZED_CHAT' });
            }, delay);
        } else {
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
        }
    }, [dispatch]);

    const setChatExpanded = useCallback(() => {
        dispatch({ type: 'SET_EXPANDED_CHAT' });
    }, [dispatch]);

    const toggleChatHistory = useCallback(async (nextState?: boolean) => {
        const willExpand = nextState !== undefined ? nextState : !state.isChatHistoryExpanded;

        if (willExpand) {
            dispatch({ type: 'SET_CHAT_HISTORY_EXPANDED' });
        } else {
            dispatch({ type: 'SET_CHAT_HISTORY_COLLAPSED' });
        }
    }, [state.isChatHistoryExpanded, state.isChatExpanded, state.isFeaturesExpanded, dispatch]);

    const closeHUD = useCallback(async () => {
        try {
            await invoke('close_main_window');
        } catch (error) {
            console.error('Failed to close window:', error);
        }
    }, [dispatch]);

    const openSettings = useCallback(async (destination?: string) => {
        try {
            await invoke('open_secondary_window');
        } catch (error) {
            console.error('Failed to open secondary window:', error);
        }
    }, [dispatch]);

    // ============================================================
    // Return API
    // ============================================================
    return {
        ...state,
        // Operations
        setChatMinimized,
        setChatExpanded,
        toggleChatHistory,
        closeHUD,
        openSettings,
    };
}