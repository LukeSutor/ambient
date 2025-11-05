'use client';

import { useEffect, useRef, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useConversationContext } from './ConversationProvider';
import { Conversation } from '@/types/conversations';
import { ChatMessage } from './types';
import { ChatStreamEvent, MemoryExtractedEvent, OcrResponseEvent, HudChatEvent } from '@/types/events';
import { MemoryEntry } from '@/types/memory';

const CONVERSATION_LIMIT = 20;
const OCR_TIMEOUT_MS = 10000;

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
    role: backendMessage.role.toLowerCase() === 'user' ? 'user' : 'assistant',
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
  const isLoadingMoreRef = useRef(false);



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
          
          // Attach memory to the message with matching message_id
          if (memory.message_id) {
            dispatch({ 
              type: 'ATTACH_MEMORY', 
              payload: { messageId: memory.message_id, memory } 
            });
          }
        });
        unlisteners.push(memoryUnlisten);

        // OCR Listener
        const ocrUnlisten = await listen<OcrResponseEvent>('ocr_response', (event) => {
          // Add OCR result and stop loading state
          const result = event.payload;
          dispatch({ type: 'CLEAR_OCR_TIMEOUT' });
          dispatch({ type: 'ADD_OCR_RESULT', payload: result });
          dispatch({ type: 'SET_OCR_LOADING', payload: false });
        });
        unlisteners.push(ocrUnlisten);

        console.log('[useConversation] Event listeners initialized');
      } catch (error) {
        console.error('[useConversation] Failed to setup events:', error);
      }
    };

    setupEvents();

    // Store cleanup function
    cleanupRef.current = () => {
      // Clear any existing OCR timeout
      dispatch({ type: 'CLEAR_OCR_TIMEOUT' });

      unlisteners.forEach((unlisten) => {
        try {
          unlisten();
        } catch (error) {
          console.error('[useConversation] Error during cleanup:', error);
        }
      });
      console.log('[useConversation] Event listeners cleaned up');
    };

    // Cleanup on unmount
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
    // Check shared initialization ref to prevent multiple initializations
    if (state.initializationRef.current) {
      return;
    }
    state.initializationRef.current = true;

    const initialize = async () => {
      console.log('[useConversation] Initializing...');
      
      // Ensure llama server is running
      try {
        await invoke<string>('spawn_llama_server');
      } catch (error) {}

      // Load the conversations list
      try {
        const conversations = await invoke<Conversation[]>('list_conversations', { 
          limit: CONVERSATION_LIMIT, offset: 0
        });
        dispatch({ type: 'SET_CONVERSATIONS', payload: conversations });
        console.log('[useConversation] Loaded conversations');
      } catch (error) {
        console.error('[useConversation] Failed to load conversations:', error);
      }
    };

    initialize();
  }, [state.initializationRef]); // Depend on the shared ref

  // ============================================================
  // Operations
  // ============================================================

  /**
   * Resets the conversation state
   */
  const resetConversation = useCallback(async (name?: string): Promise<string | null> => {
    try {
      dispatch({ type: 'SET_CONVERSATION_ID', payload: null });
      dispatch({ type: 'CLEAR_MESSAGES' });
      return null;
    } catch (error) {
      console.error('[useConversation] Failed to create conversation:', error);
      return null;
    }
  }, [dispatch]);

  /**
   * Deletes a conversation by ID
   */
  const deleteConversation = useCallback(async (id: string): Promise<void> => {
    try {
      await invoke('delete_conversation', { conversationId: id });
      dispatch({ type: 'DELETE_CONVERSATION', payload: { id } });
    } catch (error) {
      console.error('[useConversation] Failed to delete conversation:', error);
    }
  }, [dispatch]);

  /**
   * Loads a conversation by ID
   */
  const loadConversation = useCallback(async (id: string): Promise<void> => {
    try {
      const conversation = await invoke<Conversation>('get_conversation', { conversationId: id });
      dispatch({ type: 'LOAD_CONVERSATION', payload: conversation });
      await loadMessages(id);
    } catch (error) {
      console.error('[useConversation] Failed to load conversation:', error);
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
    conversationId: string | null,
    content: string,
  ): Promise<void> => {
    try {
      // Validate message
      if (!content.trim()) {
        console.warn('[useConversation] Empty message, skipping send');
        return;
      }

      // Create conversation if it doesn't exist yet (first message)
      let activeConversationId = conversationId;
      if (!activeConversationId) {
        console.log('[useConversation] Creating conversation for first message');
        const conversation = await invoke<Conversation>('create_conversation', { 
          name: null 
        });
        activeConversationId = conversation.id;
        dispatch({ type: 'SET_CONVERSATION_ID', payload: conversation.id });
        console.log('[useConversation] Created conversation:', conversation.id);
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
        ocr_responses: state.ocrResults,
        conv_id: activeConversationId,
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
  }, [dispatch, state.conversationId, state.ocrResults]);

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
   * Get all conversations
   */
  const loadMoreConversations = useCallback(async (): Promise<void> => {
    // Prevent concurrent calls
    if (isLoadingMoreRef.current || !state.hasMoreConversations) {
      return;
    }

    isLoadingMoreRef.current = true;
    
    try {
      const nextPage = state.conversationPage + 1;
      const offset = nextPage * CONVERSATION_LIMIT;
      const conversations = await invoke<Conversation[]>('list_conversations', { 
        limit: CONVERSATION_LIMIT, 
        offset 
      });
      if (conversations.length < CONVERSATION_LIMIT) {
        // No more conversations to load
        dispatch({ type: 'SET_NO_MORE_CONVERSATIONS' });
      }
      dispatch({ type: 'ADD_CONVERSATIONS', payload: conversations });
    } catch (error) {
      console.error('[useConversation] Failed to load more conversations:', error);
    } finally {
      isLoadingMoreRef.current = false;
    }
  }, [dispatch, state.conversationPage]);

  /**
   * Refresh conversations list based on current page
   */
  const refreshConversations = useCallback(async (): Promise<void> => {
    try {
      const conversations = await invoke<Conversation[]>('list_conversations', { limit: CONVERSATION_LIMIT * (state.conversationPage + 1), offset: 0 });
      dispatch({ type: 'SET_CONVERSATIONS', payload: conversations });
    } catch (error) {
      console.error('[useConversation] Failed to refresh conversations:', error);
    } finally {
    }
  }, [dispatch]);

  /**
   * Rename a conversation
   */
  const renameConversation = useCallback(async (id: string, newName: string): Promise<void> => {
    try {
      await invoke('update_conversation_name', { conversationId: id, name: newName });
      dispatch({ type: 'RENAME_CONVERSATION', payload: { id, newName } });
      await refreshConversations();
    } catch (error) {
      console.error('[useConversation] Failed to rename conversation:', error);
    }
  }, [dispatch]);

  /**
   * Dispatch an OCR capture event
   */
  const dispatchOCRCapture = useCallback(async(): Promise<void> => {
    dispatch({ type: 'CLEAR_OCR_TIMEOUT' });
    dispatch({ type: 'SET_OCR_LOADING', payload: true });
    try {
      await invoke('open_screen_selector');
      // Start a 10s timeout; if no OCR result arrives, stop loading
      const ocrTimeout = setTimeout(() => {
        console.warn('OCR capture timed out after 10s.');
        dispatch({ type: 'SET_OCR_LOADING', payload: false });
        dispatch({ type: 'CLEAR_OCR_TIMEOUT' });
      }, OCR_TIMEOUT_MS);
      dispatch({ type: 'SET_OCR_TIMEOUT', payload: ocrTimeout });
    } catch (error: any) {
      console.error('Failed to open screen selector:', error);
      dispatch({ type: 'SET_OCR_LOADING', payload: false });
      dispatch({ type: 'CLEAR_OCR_TIMEOUT' });
    }
  }, [dispatch]);

  /**
   * Delete an OCR result by its index
   */
  const deleteOCRResult = useCallback((index: number): void => {
    dispatch({ type: 'DELETE_OCR_RESULT', payload: index });
  }, [dispatch]);

  /**
   * Clear OCR results
   */
  const clearOCRResults = useCallback((): void => {
    dispatch({ type: 'CLEAR_OCR_RESULTS' });
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
    ...state,
    
    // Operations
    resetConversation,
    deleteConversation,
    loadConversation,
    loadMessages,
    sendMessage,
    clear,
    loadMoreConversations,
    renameConversation,
    dispatchOCRCapture,
    deleteOCRResult,
    clearOCRResults,
    ensureServer,
  };
}
