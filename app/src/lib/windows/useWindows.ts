'use client';

import { useCallback, useEffect, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';
import { usePathname } from 'next/navigation';
import path from 'path';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();
    const lastHeightRef = useRef<number | null>(null);
    const pathname = usePathname();

    // ============================================================
    // Effects
    // ============================================================
    useEffect(() => {
        // Resize the window based on the height of the dynamic content
        if (!state.dynamicChatContentRef.current) {
            return;
        }

        const container = state.dynamicChatContentRef.current;

        // Check if observer already exists and is observing
        if (state.resizeObserverRef.current) {
            return;
        }

        const handleResize = async () => {
            if (!container) return;

            const dimensions = await getHudDimensions();
            const newHeight = await getWindowHeight();

            // Skip if height hasn't changed
            if (newHeight === lastHeightRef.current) return;

            lastHeightRef.current = newHeight;

            try {
                await invoke('resize_hud', {
                    width: dimensions.chat_width,
                    height: newHeight
                });
            } catch (error) {
                console.error('[useWindows] Failed to resize during tracking:', error);
            }
        };

        // Set up ResizeObserver for real-time content height changes
        const resizeObserver = new ResizeObserver(() => {
            handleResize();
        });

        state.resizeObserverRef.current = resizeObserver;
        resizeObserver.observe(container);

        // Cleanup function
        return () => {
            console.log('[useWindows] Cleaning up ResizeObserver');
            if (state.resizeObserverRef.current) {
                state.resizeObserverRef.current.disconnect();
                state.resizeObserverRef.current = null;
            }
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
        };
    }, [state.dynamicChatContentRef, state.resizeObserverRef]);

    useEffect(() => {
        // Set window size based on route
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
                const height = await getWindowHeight();
                try {
                    await invoke('resize_hud', {
                        width: dimensions.chat_width,
                        height,
                    });
                } catch (error) {
                    console.error('[useWindows] Failed to resize for main HUD:', error);
                }
            }
        })();
    }, [pathname]);

    // ============================================================
    // Helpers
    // ============================================================
    const getWindowHeight = useCallback(async (featuresOverride?: boolean) => {
        // Returns the window height based on current state
        const dimensions = await getHudDimensions();

        // Both refs must exist for hud only
        if (!state.dynamicChatContentRef.current || !state.featuresRef.current) {
            return dimensions.input_bar_height;
        }

        const isFeaturesExpanded = featuresOverride !== undefined ? featuresOverride : state.isFeaturesExpanded;

        let chatHeight = Math.min(
            state.dynamicChatContentRef.current.scrollHeight,
            dimensions.chat_max_height
        );
        const featuresHeight = isFeaturesExpanded ? state.featuresRef.current.scrollHeight - 6 : 0;
        const newHeight = chatHeight + featuresHeight + dimensions.input_bar_height;
        return newHeight;
    }, [getHudDimensions]);

    // ============================================================
    // Operations
    // ============================================================
    const setLogin = useCallback(() => {
        dispatch({ type: 'SET_LOGIN' });
    }, [dispatch]);

    const setChatMinimized = useCallback(() => {
        dispatch({ type: 'SET_MINIMIZED_CHAT' });
    }, [dispatch]);

    const setChatExpanded = useCallback(() => {
        dispatch({ type: 'SET_EXPANDED_CHAT' });
    }, [dispatch]);

    const refreshHUDSize = useCallback(async () => {
        const dimensions = await getHudDimensions();
        try {
            const height = await getWindowHeight(false);
            await invoke('resize_hud', { width: dimensions.chat_width, height });
        } catch (error) {
            console.error('[useWindows] Failed to refresh HUD size:', error);
        }
    }, [dispatch, getHudDimensions]);

    const minimizeChat = useCallback(async (delay?: number) => {
        if (delay) {
            setTimeout(async () => {
                dispatch({ type: 'SET_MINIMIZED_CHAT' });
                await refreshHUDSize();
            }, delay);
        } else {
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
            await refreshHUDSize();
        }
    }, [dispatch, getHudDimensions]);

    const setFeaturesMinimized = useCallback(() => {
        dispatch({ type: 'SET_FEATURES_COLLAPSED' });
    }, [dispatch]);

    const toggleFeatures = useCallback(async (nextState?: boolean, skipDelay?: boolean) => {
        if (!state.featuresRef.current) return;

        // Determine the target state (expand vs collapse)
        const willExpand = nextState !== undefined ? nextState : !state.isFeaturesExpanded;

        // Compute dimensions once per toggle
        const dims = await getHudDimensions();

        if (willExpand) {
            dispatch({ type: 'SET_FEATURES_EXPANDED' });

            // If chat is expanded, add features height; otherwise grow from input height
            const newHeight = await getWindowHeight(true);
            try {
            await invoke('resize_hud', { width: dims.chat_width, height: newHeight });
            } catch (error) {
            console.error('Failed to resize for features expand:', error);
            }
        } else {
            dispatch({ type: 'SET_FEATURES_COLLAPSED' });

            if (state.isChatExpanded) {
            const newHeight = await getWindowHeight(false);
            setTimeout(async () => {
                try {
                await invoke('resize_hud', { width: dims.chat_width, height: newHeight });
                } catch (error) {
                console.error('Failed to resize for features collapse:', error);
                }
            }, skipDelay ? 0 : 100);
            } else {
            // When chat is not expanded, collapse back toward input height
            const newHeight = await getWindowHeight(false);
            setTimeout(async () => {
                try {
                await invoke('resize_hud', { width: dims.chat_width, height: newHeight });
                } catch (error) {
                console.error('Failed to resize for features collapse:', error);
                }
            }, skipDelay ? 0 : 100);
            }
        }

    }, [state.isFeaturesExpanded, state.isChatExpanded, dispatch, getHudDimensions]);

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
        dispatch({ type: 'OPEN_SETTINGS', payload: destination });
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
        setLogin,
        setChatMinimized,
        setChatExpanded,
        refreshHUDSize,
        minimizeChat,
        setFeaturesMinimized,
        toggleFeatures,
        toggleChatHistory,
        closeHUD,
        openSettings,
    };
}