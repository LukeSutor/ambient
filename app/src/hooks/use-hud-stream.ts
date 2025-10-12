'use client';

import { MutableRefObject, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { SettingsService } from '@/lib/settings-service';
import type { HudDimensions } from '@/types/settings';
import type { ChatStreamEvent, HudChatEvent, OcrResponseEvent, MemoryExtractedEvent } from '@/types/events';
import { MemoryEntry } from '@/types/memory';

export interface Conversation {
  id: string;
  name: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

export function useHudStream({
  isExpandedRef,
  setHudDimensions,
  hudDimensionsRef,
  setMessages,
  setIsLoading,
  setIsStreaming,
  setOcrResults,
  setOcrLoading,
  ocrTimeoutRef,
  currentConversationIdRef,
  setCurrentConversationId,
  messagesEndRef,
}: {
  isExpandedRef: MutableRefObject<boolean>;
  setHudDimensions: (d: HudDimensions | null) => void;
  hudDimensionsRef: MutableRefObject<HudDimensions | null>;
  setMessages: React.Dispatch<React.SetStateAction<{ role: 'user' | 'assistant'; content: string; memory: MemoryEntry | null }[]>>;
  setIsLoading: (b: boolean) => void;
  setIsStreaming: (b: boolean) => void;
  setOcrResults: React.Dispatch<React.SetStateAction<OcrResponseEvent[]>>;
  setOcrLoading: (b: boolean) => void;
  ocrTimeoutRef: MutableRefObject<ReturnType<typeof setTimeout> | null>;
  currentConversationIdRef: MutableRefObject<string | null>;
  setCurrentConversationId: (id: string | null) => void;
  messagesEndRef: MutableRefObject<HTMLDivElement | null>;
}) {
  const streamContentRef = useRef('');

  function extractThinkingContent(text: string) {
    const thinkStartIndex = text.indexOf('<think>');
    const thinkEndIndex = text.indexOf('</think>');
    let cleanText = text;
    if (thinkStartIndex !== -1) {
      cleanText = thinkEndIndex !== -1
        ? text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8)
        : text.substring(0, thinkStartIndex);
    }
    return cleanText;
  }

  // Bootstrapping: dimensions, listeners, llama server, conversation, OCR
  useEffect(() => {
    const loadHudDimensions = async () => {
      try {
        const dimensions = await SettingsService.getHudDimensions();
        if (!hudDimensionsRef.current || JSON.stringify(dimensions) !== JSON.stringify(hudDimensionsRef.current)) {
          setHudDimensions(dimensions);
          return true;
        }
        return false;
      } catch (error) {
        console.error('Failed to load HUD dimensions:', error);
        const fallback: HudDimensions = { width: 500, collapsed_height: 60, expanded_height: 350 };
        setHudDimensions(fallback);
        return true;
      }
    };

    let unlistenSettings: UnlistenFn | null = null;
    let unlistenStream: UnlistenFn | null = null;
    let unlistenOCR: UnlistenFn | null = null;
    let unlistenMemoryExtracted: UnlistenFn | null = null;

    (async () => {
      // Dimensions and settings listener
      await loadHudDimensions();
      try {
        unlistenSettings = await listen('settings_changed', async () => {
          const changed = await loadHudDimensions();
          if (changed) {
            try {
              await invoke('refresh_hud_window_size', { label: 'floating-hud', isExpanded: isExpandedRef.current });
            } catch (error) {
              console.error('Failed to refresh HUD window size:', error);
            }
          }
        });
      } catch (err) {
        console.error('Failed to set up settings listener:', err);
      }

      // Ensure llama server running and create conversation
      try {
        await invoke<string>('spawn_llama_server');
      } catch (e) {
        console.warn('spawn_llama_server failed or not available:', e);
      }

      try {
        const newConv = await invoke<Conversation>('create_conversation', { name: null });
        setCurrentConversationId(newConv.id);
        currentConversationIdRef.current = newConv.id;
        const existing = await invoke<any[]>('get_messages', { conversationId: newConv.id });
        const mapped = existing.map((m) => ({
          role: m.role === 'user' ? ('user' as const) : ('assistant' as const),
          content: extractThinkingContent(m.content),
          memory: m.memory ? (m.memory as MemoryEntry) : null,
        }));
        setMessages(mapped);
      } catch (err) {
        console.error('Failed to init conversation for HUD:', err);
      }

      // Stream listener
      try {
        unlistenStream = await listen<ChatStreamEvent>('chat_stream', (event) => {
          const { delta, full_response, is_finished, conv_id } = event.payload;
          if (conv_id !== currentConversationIdRef.current) return;

          if (is_finished) {
            const finalText = extractThinkingContent(full_response ?? streamContentRef.current);
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) next[lastIdx] = { ...next[lastIdx], content: finalText };
              return next;
            });
            setIsLoading(false);
            setIsStreaming(false);
            streamContentRef.current = '';
            return;
          }

          if (delta) {
            streamContentRef.current += delta;
            const clean = extractThinkingContent(streamContentRef.current);
            setMessages((prev) => {
              const next = [...prev];
              const idx = [...next].reverse().findIndex((m) => m.role === 'assistant');
              const lastIdx = idx >= 0 ? next.length - 1 - idx : -1;
              if (lastIdx >= 0) {
                next[lastIdx] = { ...next[lastIdx], content: clean };
              } else {
                next.push({ role: 'assistant', content: clean, memory: null });
              }
              return next;
            });
            queueMicrotask(() => messagesEndRef.current?.scrollIntoView({ behavior: 'smooth', block: 'end' }));
          }
        });
      } catch (err) {
        console.error('Failed to set up chat_stream listener:', err);
      }

      // OCR listener
      try {
        unlistenOCR = await listen<OcrResponseEvent>('ocr_response', (event) => {
          const result = event.payload as OcrResponseEvent;
          if (!result.success) console.error('OCR failed');
          if (ocrTimeoutRef.current) {
            clearTimeout(ocrTimeoutRef.current);
            ocrTimeoutRef.current = null;
          }
          setOcrResults((prev) => [...prev, result]);
          setOcrLoading(false);
        });
      } catch (err) {
        console.error('Failed to set up OCR listener:', err);
      }

      // Memory extracted listener
      try {
        unlistenMemoryExtracted = await listen<MemoryExtractedEvent>('memory_extracted', (event) => {
          const { memory } = event.payload;
          console.log('Memory extracted:', memory);
          // Update the messages so that the message with the matching message_id (if any) gets the memory attached
          setMessages((prev) => {
            const next = prev.map((m) => {
              if (m.role === 'user' && memory.message_id && m.content) {
                return { ...m, memory };
              }
              return m;
            });
            return next;
          });
        });
      } catch (err) {
        console.error('Failed to set up memory_extracted listener:', err);
      }

      // Transparent background class
      if (typeof document !== 'undefined') {
        document.documentElement.classList.add('hud-transparent');
        document.body.classList.add('hud-transparent');
      }
    })();

    return () => {
      if (typeof document !== 'undefined') {
        document.documentElement.classList.remove('hud-transparent');
        document.body.classList.remove('hud-transparent');
      }
      try { unlistenStream?.(); } catch {}
      try { unlistenSettings?.(); } catch {}
      try { unlistenOCR?.(); } catch {}
      try { unlistenMemoryExtracted?.(); } catch {}
      if (ocrTimeoutRef.current) {
        clearTimeout(ocrTimeoutRef.current);
        ocrTimeoutRef.current = null;
      }
    };
  }, []);

  // Convenience actions that were in page.tsx
  async function closeWindow() {
    try {
      await invoke('close_floating_window', { label: 'floating-hud' });
    } catch (error) {
      console.error('Failed to close window:', error);
      try {
        const currentWindow = getCurrentWebviewWindow();
        await currentWindow.close();
      } catch (altError) {
        console.error('Direct close method also failed:', altError);
      }
    }
  }

  async function createNewConversation() {
    try {
      const newConv = await invoke<Conversation>('create_conversation', { name: null });
      setCurrentConversationId(newConv.id);
      currentConversationIdRef.current = newConv.id;
      return newConv.id;
    } catch (err) {
      console.error('Error creating conversation:', err);
      return null;
    }
  }

  return { createNewConversation, closeWindow };
}

export default useHudStream;
