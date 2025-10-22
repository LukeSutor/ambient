'use client';

import { useCallback, useEffect, useRef } from 'react';
import { RefObject } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useWindowsContext } from './WindowsProvider';
import { useSettings } from '../settings/useSettings';

export function useWindows() {
    const { state, dispatch } = useWindowsContext();
    const { getHudDimensions } = useSettings();
    const resizeTimeoutRef = useRef<number | null>(null);
    const lastHeightRef = useRef<number | null>(null);

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

    const expandChat = useCallback(async (messagesContainerRef: RefObject<HTMLDivElement | null>) => {
        const dimensions = getHudDimensions();
        try {
            if (messagesContainerRef.current) {
                const scrollHeight = messagesContainerRef.current.scrollHeight;
                let new_height = Math.min(dimensions.chat_max_height, scrollHeight) + dimensions.input_bar_height;
                console.log("EXPANDING HUD")
                await invoke('resize_hud', { width: dimensions.chat_width, height: new_height });
            }
            dispatch({ type: 'SET_EXPANDED_CHAT' });
        } catch (error) {
            console.error('[useWindows] Failed to expand chat HUD:', error);
        }
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
            console.log("no container");
            return;
        }

        const container = messagesContainerRef.current;

        const handleResize = async () => {
            if (!container) return;

            const contentHeight = container.scrollHeight;

            const totalHeight = contentHeight + 6; // Add small padding
            
            // Skip if height hasn't changed
            if (totalHeight === lastHeightRef.current) return;

            lastHeightRef.current = totalHeight;

            // Calculate window height: content height (capped at max) + input bar
            const windowHeight = Math.min(
                totalHeight,
                dimensions.chat_max_height
            ) + dimensions.input_bar_height;

            try {
                console.log("resizing hud to height:", windowHeight);
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
            if (resizeTimeoutRef.current) {
                clearTimeout(resizeTimeoutRef.current);
                resizeTimeoutRef.current = null;
            }
        };
    }, [getHudDimensions]);

    // ============================================================
    // Return API
    // ============================================================
    return {
        isLogin: state.isLogin,
        isExpanded: state.isExpanded,
        setLogin,
        minimizeChat,
        expandChat,
        trackContentAndResize
    };
}