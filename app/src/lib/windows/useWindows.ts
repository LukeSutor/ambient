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

    // ============================================================
    // Return API
    // ============================================================
    return {
        isLogin: state.isLogin,
        isChatExpanded: state.isChatExpanded,
        setLogin,
        setMinimizedChat,
        setExpandedChat,
        refreshHUDSize,
        minimizeChat,
        trackContentAndResize
    };
}