'use client';

import { useCallback, useEffect, useRef } from 'react';
import { RefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();
    const lastHeightRef = useRef<number | null>(null);

    // ============================================================
    // Operations
    // ============================================================
    const setLogin = useCallback(() => {
        dispatch({ type: 'SET_LOGIN' });
    }, [dispatch]);

    const setMinimizedChat = useCallback(() => {
        dispatch({ type: 'SET_MINIMIZED_CHAT' });
    }, [dispatch]);

    const setExpandedChat = useCallback(() => {
        dispatch({ type: 'SET_EXPANDED_CHAT' });
    }, [dispatch]);

    const refreshHUDSize = useCallback(async () => {
        const dimensions = getHudDimensions();
        try {
            await invoke('resize_hud', { width: dimensions.chat_width, height: dimensions.input_bar_height });
        } catch (error) {
            console.error('[useWindows] Failed to refresh HUD size:', error);
        }
    }, [dispatch, getHudDimensions]);

    const minimizeChat = useCallback(async () => {
        dispatch({ type: 'SET_MINIMIZED_CHAT' });
        await refreshHUDSize();
    }, [dispatch, getHudDimensions]);

    /**
     * Track content height changes and dynamically resize window
     * Uses ResizeObserver for real-time height monitoring during streaming
     * 
     * @param messagesContainerRef - Reference to the messages container element
     */
    const trackContentAndResize = useCallback((
        messagesContainerRef: RefObject<HTMLDivElement | null>,
    ) => {
        const dimensions = getHudDimensions();

        if (!messagesContainerRef.current) {
            return;
        }

        const container = messagesContainerRef.current;

        const handleResize = async () => {
            if (!container) return;

            const contentHeight = container.scrollHeight;

            const totalHeight = contentHeight + 6;

            // Skip if height hasn't changed
            if (totalHeight === lastHeightRef.current) return;

            lastHeightRef.current = totalHeight;

            // Calculate window height: content height (capped at max) + input bar
            const windowHeight = Math.min(
                totalHeight,
                dimensions.chat_max_height
            ) + dimensions.input_bar_height;

            try {
                //TODO: Fix bug with cutting off expanded features list
                await invoke('resize_hud', {
                    width: dimensions.chat_width,
                    height: windowHeight
                });
            } catch (error) {
                console.error('[useWindows] Failed to resize during tracking:', error);
            }
        };

        // Set up ResizeObserver for real-time content height changes
        const resizeObserver = new ResizeObserver(() => {
            handleResize();
        });

        resizeObserver.observe(container);

        // Cleanup function
        return () => {
            resizeObserver.disconnect();
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
        };
    }, [getHudDimensions]);

    const toggleFeatures = useCallback(async (
        featuresRef: RefObject<HTMLDivElement | null>
    ) => {
        if (!featuresRef.current) return;

        const isExpanded = state.isFeaturesExpanded;

        if (isExpanded) {
            dispatch({ type: 'SET_FEATURES_COLLAPSED' });
            
            if (state.isChatExpanded) {
                //TODO: shrink to previous chat size if needed
            } else {
                // Shrink back if not expanded
                setTimeout(async () => {
                    await refreshHUDSize();
                }, 250);
            }
        } else {
            dispatch({ type: 'SET_FEATURES_EXPANDED' });

            if (state.isChatExpanded) {
                //TODO: expand to fit features if needed
            } else {
                // Expand to fit features
                const featuresHeight = featuresRef.current.scrollHeight;
                const dimensions = getHudDimensions();
                const newHeight = dimensions.input_bar_height + featuresHeight - 6;
                
                try {
                    await invoke('resize_hud', {
                        width: dimensions.chat_width,
                        height: newHeight
                    });
                } catch (error) {
                    console.error('Failed to resize for features expansion:', error);
                }
            }
        }

    }, [state.isFeaturesExpanded, state.isChatExpanded, dispatch, getHudDimensions]);

    const closeHUD = useCallback(async () => {
        try {
            await invoke('close_floating_window', { label: 'floating-hud' });
        } catch (error) {
            console.error('Failed to close window:', error);
        }
    }, [dispatch]);

    const openSettings = useCallback(async (destination?: string) => {
        dispatch({ type: 'OPEN_SETTINGS', payload: destination });
        try {
            await invoke('open_main_window');
        } catch (error) {
            console.error('Failed to open main window:', error);
        }
    }, [dispatch]);

    // ============================================================
    // Return API
    // ============================================================
    return {
        ...state,
        setLogin,
        setMinimizedChat,
        setExpandedChat,
        refreshHUDSize,
        minimizeChat,
        trackContentAndResize,
        toggleFeatures,
        closeHUD,
        openSettings,
    };
}