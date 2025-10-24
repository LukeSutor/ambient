'use client';

import { useEffect, useRef, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useConversationContext } from './ConversationProvider';
import { ChatMessage, Conversation } from './types';
import { ChatStreamEvent, MemoryExtractedEvent, OcrResponseEvent, HudChatEvent } from '@/types/events';
import { MemoryEntry } from '@/types/memory';

/**
 * Extracts and removes <think> tags from LLM responses
 */
function extractThinkingContent(text: string): string {
  const thinkStartIndex = text.indexOf('<think>');
  const thinkEndIndex = text.indexOf('</think>');
  
  let cleanText = text;
  
  if (thinkStartIndex !== -1) {
    if (thinkEndIndex !== -1) {
      // Remove complete thinking block
      cleanText = text.substring(0, thinkStartIndex) + text.substring(thinkEndIndex + 8);
    } else {
      // Remove incomplete thinking block
      cleanText = text.substring(0, thinkStartIndex);
    }
  }
  
  return cleanText;
}

/**
 * Transforms a backend message format to frontend ChatMessage format
 */
function transformBackendMessage(backendMessage: any): ChatMessage {
  return {
    id: backendMessage.id,
    role: backendMessage.role === 'user' ? 'user' : 'assistant',
    content: extractThinkingContent(backendMessage.content),
    memory: backendMessage.memory ? (backendMessage.memory as MemoryEntry) : null,
    timestamp: backendMessage.timestamp,
  };
}

/**
 * Creates a new user message
 */
function createUserMessage(content: string, memory: MemoryEntry | null = null): ChatMessage {
  return {
    id: crypto.randomUUID(),
    role: 'user',
    content,
    memory,
    timestamp: new Date().toISOString(),
  };
}

/**
 * Main conversation hook - provides all conversation functionality
 * 
 * @param messagesEndRef - Optional ref for auto-scrolling to bottom on new messages
 * @returns Conversation state and operations
 */
export function useConversation(messagesEndRef?: React.RefObject<HTMLDivElement | null>) {
  const { state, dispatch } = useConversationContext();
  const cleanupRef = useRef<(() => void) | null>(null);
  const initializedRef = useRef(false);

  // ============================================================
  // Event Listeners Setup
  // ============================================================

  useEffect(() => {
    let isMounted = true;
    const unlisteners: UnlistenFn[] = [];
    let streamContent = '';

    const setupEvents = async () => {
      // Clean up previous listeners
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }

      if (!isMounted) return;

      try {
        // Stream Listener
        const streamUnlisten = await listen<ChatStreamEvent>('chat_stream', (event) => {
          const { delta, full_response, is_finished, conv_id } = event.payload;

          // Filter by conversation ID to prevent corruption
          if (conv_id !== state.conversationId) {
            console.log('[useConversation] Ignoring stream event for different conversation');
            return;
          }

          if (is_finished) {
            // Stream is complete
            const finalText = extractThinkingContent(full_response ?? streamContent);
            dispatch({ type: 'FINALIZE_STREAM', payload: finalText });
            streamContent = '';
            return;
          }

          if (delta) {
            // Accumulate stream content
            streamContent += delta;
            const cleanContent = extractThinkingContent(streamContent);
            dispatch({ type: 'UPDATE_STREAMING_CONTENT', payload: cleanContent });

            // Auto-scroll to bottom
            if (messagesEndRef?.current) {
              queueMicrotask(() => {
                messagesEndRef.current?.scrollIntoView({ 
                  behavior: 'smooth', 
                  block: 'end' 
                });
              });
            }
          }
        });
        unlisteners.push(streamUnlisten);

        // Memory Listener
        const memoryUnlisten = await listen<MemoryExtractedEvent>('memory_extracted', (event) => {
          const { memory } = event.payload;
          
          console.log('[useConversation] Memory extracted:', memory);

          // Attach memory to the message with matching message_id
          if (memory.message_id) {
            dispatch({ 
              type: 'ATTACH_MEMORY', 
              payload: { messageId: memory.message_id, memory } 
            });
          }
        });
        unlisteners.push(memoryUnlisten);

        console.log('[useConversation] Event listeners initialized');
      } catch (error) {
        console.error('[useConversation] Failed to setup events:', error);
      }
    };

    setupEvents();

    // Store cleanup function
    cleanupRef.current = () => {
      unlisteners.forEach((unlisten) => {
        try {
          unlisten();
        } catch (error) {
          console.error('[useConversation] Error during cleanup:', error);
        }
      });
      console.log('[useConversation] Event listeners cleaned up');
    };

    // Cleanup on unmount or conversationId change
    return () => {
      isMounted = false;
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, [state.conversationId, dispatch, messagesEndRef]);

  // ============================================================
  // Initialization Effect
  // ============================================================

  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    const initialize = async () => {
      console.log('[useConversation] Initializing...');
      
      // Ensure llama server is running
      try {
        await invoke<string>('spawn_llama_server');
      } catch (error) {
        console.warn('[useConversation] spawn_llama_server warning:', error);
      }

      // Create initial conversation if none exists
      if (!state.conversationId) {
        try {
          const conversation = await invoke<Conversation>('create_conversation', { 
            name: null 
          });
          dispatch({ type: 'SET_CONVERSATION_ID', payload: conversation.id });
          dispatch({ type: 'CLEAR_MESSAGES' });
          console.log('[useConversation] Created initial conversation:', conversation.id);
        } catch (error) {
          console.error('[useConversation] Failed to create conversation:', error);
        }
      }
    };

    initialize();
  }, []); // Only run once on mount

  // ============================================================
  // Operations
  // ============================================================

  /**
   * Creates a new conversation
   */
  const createNew = useCallback(async (name?: string): Promise<string | null> => {
    try {
      const conversation = await invoke<Conversation>('create_conversation', { 
        name: name || null 
      });
      dispatch({ type: 'SET_CONVERSATION_ID', payload: conversation.id });
      dispatch({ type: 'CLEAR_MESSAGES' });
      return conversation.id;
    } catch (error) {
      console.error('[useConversation] Failed to create conversation:', error);
      return null;
    }
  }, [dispatch]);

  /**
   * Loads messages for a specific conversation
   */
  const loadMessages = useCallback(async (conversationId: string): Promise<void> => {
    try {
      const backendMessages = await invoke<any[]>('get_messages', { 
        conversationId 
      });
      
      const messages = backendMessages.map(transformBackendMessage);
      dispatch({ type: 'LOAD_MESSAGES', payload: messages });
    } catch (error) {
      console.error('[useConversation] Failed to load messages:', error);
    }
  }, [dispatch]);

  /**
   * Sends a message with optional OCR context
   */
  const sendMessage = useCallback(async (
    conversationId: string,
    content: string,
    ocrResults: OcrResponseEvent[] = []
  ): Promise<void> => {
    try {
      // Validate message
      if (!content.trim()) {
        console.warn('[useConversation] Empty message, skipping send');
        return;
      }

      if (!conversationId) {
        throw new Error('No conversation ID provided');
      }

      // Create user message with ID and timestamp
      const userMessage = createUserMessage(content);

      // Start user message with empty content (for animation)
      dispatch({ 
        type: 'START_USER_MESSAGE', 
        payload: { id: userMessage.id, timestamp: userMessage.timestamp } 
      });

      // Use requestAnimationFrame to ensure the empty state is rendered first
      requestAnimationFrame(() => {
        // Then fill in the content to trigger the grid animation
        dispatch({ 
          type: 'FINALIZE_USER_MESSAGE', 
          payload: { id: userMessage.id, content: userMessage.content } 
        });
      });

      // Start assistant message placeholder
      dispatch({ type: 'START_ASSISTANT_MESSAGE' });
      dispatch({ type: 'SET_LOADING', payload: true });
      dispatch({ type: 'SET_STREAMING', payload: true });

      // Send to backend (streaming will be handled by event listeners)
      const hudChatEvent: HudChatEvent = {
        text: content,
        ocr_responses: ocrResults,
        conv_id: conversationId,
        timestamp: Date.now().toString(),
        message_id: userMessage.id,
      };

      await invoke<string>('handle_hud_chat', {
        event: hudChatEvent,
      });
    } catch (error) {
      console.error('[useConversation] Failed to send message:', error);
      
      // Remove the placeholder assistant message on error
      dispatch({ type: 'FINALIZE_STREAM', payload: '[Error generating response]' });
      dispatch({ type: 'SET_LOADING', payload: false });
      dispatch({ type: 'SET_STREAMING', payload: false });
    }
  }, [dispatch]);

  /**
   * Clears all messages in the current conversation
   */
  const clear = useCallback((delay?: number): void => {
    if (delay) {
      setTimeout(() => {
        dispatch({ type: 'CLEAR_MESSAGES' });
      }, delay);
    } else {
      dispatch({ type: 'CLEAR_MESSAGES' });
    }
  }, [dispatch]);

  /**
   * Ensures the llama server is running
   */
  const ensureServer = useCallback(async (): Promise<void> => {
    try {
      await invoke<string>('spawn_llama_server');
    } catch (error) {
      console.warn('[useConversation] Failed to ensure server running:', error);
    }
  }, []);

  // ============================================================
  // Return API
  // ============================================================

  return {
    // State
    conversationId: state.conversationId,
    messages: state.messages,
    isStreaming: state.isStreaming,
    isLoading: state.isLoading,
    streamingContent: state.streamingContent,
    
    // Operations
    createNew,
    loadMessages,
    sendMessage,
    clear,
    ensureServer,
  };
}
