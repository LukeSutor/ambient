'use client';

import { useCallback, useEffect, useRef } from 'react';
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
    // useGSAP(() => {
    //     if (!state.dynamicChatContentRef.current) return;
    //     const container = state.dynamicChatContentRef.current;
    //     if (state.isChatExpanded || state.isChatHistoryExpanded) {
    //         console.log('Expanding chat area animation');
    //         gsap.set(container, { padding: '12px', scale: 0.95, height: 'auto' });
    //         gsap.to(container, {
    //             opacity: 1,
    //             scale: 1,
    //             duration: 1,
    //             ease: 'back.out(1.2)',
    //         });
    //     } else {
    //         console.log('Collapsing chat area animation');
    //         gsap.to(container, {
    //             opacity: 0,
    //             scale: 0.95,
    //             height: 0,
    //             duration: 0.25,
    //             padding: 0,
    //             ease: 'power2.inOut',
    //             onComplete: () => { gsap.set(container, { scale: 0, height: 'auto' }); }
    //         });
    //     }
    // }, [state.isChatExpanded, state.isChatHistoryExpanded]);

    useEffect(() => {
        // Resize the window based on the height of the dynamic content
        if (!state.dynamicChatContentRef.current) {
            return;
        }

        const container = state.dynamicChatContentRef.current;

        const handleResize = async () => {
            if (!container) return;

            const dimensions = await getHudDimensions();
            const newHeight = await getWindowHeight();
            console.log('new height', newHeight);

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
    }, [state.dynamicChatContentRef]);

    // ============================================================
    // Helpers
    // ============================================================
    const getWindowHeight = useCallback(async (expandedOverride?: boolean, featuresOverride?: boolean) => {
        // Returns the window height based on current state
        //TODO: update code to use this function properly
        const dimensions = await getHudDimensions();

        if (!state.dynamicChatContentRef.current || !state.featuresRef.current) {
            return dimensions.input_bar_height;
        }

        // const isExpanded = expandedOverride !== undefined ? expandedOverride : state.isChatExpanded;
        const isFeaturesExpanded = featuresOverride !== undefined ? featuresOverride : state.isFeaturesExpanded;

        // console.log('isExpanded', isExpanded, 'isFeatures', isFeaturesExpanded);

        // if (isExpanded) {
            // Calculate height based on chat content and features panel
            const chatHeight = Math.min(
                state.dynamicChatContentRef.current.scrollHeight,
                dimensions.chat_max_height
            ) + 6;
            const featuresHeight = isFeaturesExpanded ? state.featuresRef.current.scrollHeight - 6 : 0;
            const newHeight = chatHeight + featuresHeight + dimensions.input_bar_height;
            return newHeight;
        // } else {
        //     const featuresHeight = isFeaturesExpanded ? state.featuresRef.current.scrollHeight - 6 : 0;
        //     return dimensions.input_bar_height + featuresHeight;
        // }
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
        if (!state.dynamicChatContentRef?.current) {
            return;
        }

        const container = state.dynamicChatContentRef.current;

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
            const newHeight = await getWindowHeight(state.isChatExpanded, true);
            try {
            await invoke('resize_hud', { width: dims.chat_width, height: newHeight });
            } catch (error) {
            console.error('Failed to resize for features expand:', error);
            }
        } else {
            dispatch({ type: 'SET_FEATURES_COLLAPSED' });

            if (state.isChatExpanded) {
            const newHeight = await getWindowHeight(true, false);
            setTimeout(async () => {
                try {
                await invoke('resize_hud', { width: dims.chat_width, height: newHeight });
                } catch (error) {
                console.error('Failed to resize for features collapse:', error);
                }
            }, skipDelay ? 0 : 100);
            } else {
            // When chat is not expanded, collapse back toward input height
            const newHeight = await getWindowHeight(false, false);
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
        setChatMinimized,
        setChatExpanded,
        refreshHUDSize,
        minimizeChat,
        trackContentAndResize,
        setFeaturesMinimized,
        toggleFeatures,
        toggleChatHistory,
        closeHUD,
        openSettings,
    };
}