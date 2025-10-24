'use client';

import { useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import gsap from 'gsap';
import { useGSAP } from '@gsap/react';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();
    const lastHeightRef = useRef<number | null>(null);

    // ============================================================
    // Effects
    // ============================================================
    useGSAP(() => {
        if (!state.messagesContainerRef.current) return;
        const container = state.messagesContainerRef.current;
        if (state.isChatExpanded) {
            console.log('Expanding chat area animation');
            gsap.set(container, { padding: '12px', scale: 0.95 });
            gsap.to(container, {
                opacity: 1,
                scale: 1,
                duration: 1,
                ease: 'back.out(1.2)',
            });
        } else {
            console.log('Collapsing chat area animation');
            gsap.to(container, {
                opacity: 1,
                scale: 0,
                duration: 0.25,
                padding: 0,
                ease: 'power2.inOut',
            });
        }
    }, [state.isChatExpanded]);

    // ============================================================
    // Helpers
    // ============================================================
    const getWindowHeight = useCallback(async (expandedOverride?: boolean, featuresOverride?: boolean) => {
        // Returns the window height based on current state
        //TODO: update code to use this function properly
        const dimensions = await getHudDimensions();

        if (!state.messagesContainerRef.current || !state.featuresRef.current) {
            console.log('[useWindows] Refs not set, returning input bar height');
            return dimensions.input_bar_height;
        }

        const isExpanded = expandedOverride !== undefined ? expandedOverride : state.isChatExpanded;
        const isFeaturesExpanded = featuresOverride !== undefined ? featuresOverride : state.isFeaturesExpanded;

        if (isExpanded) {
            // Calculate height based on chat content and features panel
            const chatHeight = Math.min(
                state.messagesContainerRef.current.scrollHeight,
                dimensions.chat_max_height
            ) + 6;
            const featuresHeight = isFeaturesExpanded ? state.featuresRef.current.scrollHeight - 6 : 0;
            const newHeight = chatHeight + featuresHeight + dimensions.input_bar_height;
            return newHeight;
        } else {
            return dimensions.input_bar_height;
        }
    }, [getHudDimensions]);

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
        const dimensions = await getHudDimensions();
        try {
            const height = await getWindowHeight(false, false);
            console.log('[useWindows] Refreshing HUD size to', { width: dimensions.chat_width, height });
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

    /**
     * Track content height changes and dynamically resize window
     * Uses ResizeObserver for real-time height monitoring during streaming
     */
    const trackContentAndResize = useCallback(() => {
        if (!state.messagesContainerRef.current) {
            return;
        }

        const container = state.messagesContainerRef.current;

        const handleResize = async () => {
            if (!container) return;

            const dimensions = await getHudDimensions();
            const newHeight = await getWindowHeight(true);

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

        resizeObserver.observe(container);

        // Cleanup function
        return () => {
            resizeObserver.disconnect();
            dispatch({ type: 'SET_MINIMIZED_CHAT' });
        };
    }, [getHudDimensions]);

    const toggleFeatures = useCallback(async (newState?: boolean) => {
        if (!state.featuresRef.current) return;

        const isExpanded = newState !== undefined ? !newState : state.isFeaturesExpanded;

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
                const featuresHeight = state.featuresRef.current.scrollHeight;
                const dimensions = await getHudDimensions();
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
        // Operations
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