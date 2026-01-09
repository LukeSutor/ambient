'use client';

import { useEffect, useRef, useCallback } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { useConversationContext } from './ConversationProvider';
import { Conversation, Message, Role } from '@/types/conversations';
import { ChatMessage } from './types';
import { ChatStreamEvent, MemoryExtractedEvent, OcrResponseEvent, HudChatEvent, ComputerUseUpdateEvent } from '@/types/events';
import { MemoryEntry } from '@/types/memory';
import { 
  startComputerUseSession, 
  createConversation, 
  sendMessage as sendChatApiMessage, 
  deleteConversation as deleteApiConversation,
  ensureLlamaServerRunning
} from './api';

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
function transformBackendMessage(backendMessage: Message): ChatMessage {
  return {
    message: backendMessage,
    reasoningMessages: [],
    memory: null,
  };
}

/**
 * Creates a new user message
 */
function createUserMessage(content: string, conversationId: string, memory: MemoryEntry | null = null): ChatMessage {
  const message: Message = {
    id: crypto.randomUUID(),
    conversation_id: conversationId,
    role: 'user' as Role,
    content,
    timestamp: new Date().toISOString(),
  };
  return {
    message,
    reasoningMessages: [],
    memory,
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

        // Computer Use Listener
        const computerUseUnlisten = await listen<ComputerUseUpdateEvent>('computer_use_update', (event) => {
          // Create message from event
          const chatMessage: ChatMessage = {
            message: event.payload.message,
            reasoningMessages: [],
            memory: null,
          };
          
          if (event.payload.status === 'completed') {
            dispatch({ type: 'ADD_CHAT_MESSAGE', payload: chatMessage });
            dispatch({ type: 'SET_STREAMING', payload: false });
            dispatch({ type: 'SET_LOADING', payload: false });
          } else {
            dispatch({ type: 'ADD_REASONING_MESSAGE', payload: chatMessage });
          }

        });
        unlisteners.push(computerUseUnlisten);

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
          // Add stop loading state and add successful OCR result
          const result = event.payload;
          dispatch({ type: 'CLEAR_OCR_TIMEOUT' });
          dispatch({ type: 'SET_OCR_LOADING', payload: false });
          if (result.success) {
            dispatch({ type: 'ADD_OCR_RESULT', payload: result });
          }
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
      await ensureLlamaServerRunning();

      // Load the conversations list
      try {
        const conversations = await invoke<Conversation[]>('list_conversations', { 
          limit: CONVERSATION_LIMIT, offset: 0
        });
        dispatch({ type: 'SET_CONVERSATIONS', payload: conversations });
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
  const resetConversation = useCallback(async (delay?: number): Promise<string | null> => {
    try {
      dispatch({ type: 'SET_CONVERSATION_ID', payload: null });
      dispatch({ type: 'SET_CONVERSATION_TYPE', payload: 'chat' });
      if (delay && delay > 0) {
        setTimeout(() => {
          dispatch({ type: 'CLEAR_MESSAGES' });
        }, delay);
      } else {
        dispatch({ type: 'CLEAR_MESSAGES' });
      }
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
      await deleteApiConversation(id);
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
      await loadMessages(conversation);
    } catch (error) {
      console.error('[useConversation] Failed to load conversation:', error);
    }
  }, [dispatch]);

  /**
   * Loads messages for a specific conversation
   */
  const loadMessages = useCallback(async (conversation: Conversation): Promise<void> => {
    console.log('[useConversation] Loading messages for conversation:', conversation.id);
    try {
      const backendMessages = await invoke<Message[]>('get_messages', { 
        conversationId: conversation.id
      });
      const messages = backendMessages.map(transformBackendMessage);
      // Load messages depending on conversation type
      if (state.conversationType === 'computer_use') {
        // Load all but the final assistant message as reasoning messages
        const reasoningMessages = messages.slice(0, -1);
        const finalMessage = messages[messages.length - 1];
        dispatch({ type: 'LOAD_REASONING_MESSAGES', payload: reasoningMessages });
        dispatch({ type: 'LOAD_MESSAGES', payload: finalMessage ? [finalMessage] : [] });
      } else {
        dispatch({ type: 'LOAD_MESSAGES', payload: messages });
      }
    } catch (error) {
      console.error('[useConversation] Failed to load messages:', error);
    }
  }, [dispatch, state.conversationType]);

  /**
   * Sends a message
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
        const conversation = await createConversation(undefined, state.conversationType);
        activeConversationId = conversation.id;
        dispatch({ type: 'SET_CONVERSATION_ID', payload: conversation.id });
        console.log('[useConversation] Created conversation:', conversation);
      }

      // Create user message with ID and timestamp
      const userMessage = createUserMessage(content, activeConversationId);

      // Start user message with empty content (for animation)
      dispatch({ 
        type: 'START_USER_MESSAGE', 
        payload: { id: userMessage.message.id, conversationId: activeConversationId, timestamp: userMessage.message.timestamp } 
      });

      // Use requestAnimationFrame to ensure the empty state is rendered first
      requestAnimationFrame(() => {
        // Then fill in the content to trigger the grid animation
        dispatch({ 
          type: 'FINALIZE_USER_MESSAGE', 
          payload: { id: userMessage.message.id, content: userMessage.message.content } 
        });
      });

      dispatch({ type: 'SET_LOADING', payload: true });
      dispatch({ type: 'SET_STREAMING', payload: true });
      
      // Send hud chat or computer use event
      if (state.conversationType === 'computer_use') {
        startComputerUseSession(activeConversationId, content);
      } else {
        dispatch({ type: 'START_ASSISTANT_MESSAGE', payload: { conversationId: activeConversationId } });
        await sendChatApiMessage(
          activeConversationId,
          content,
          state.ocrResults,
          userMessage.message.id
        );
      }
    } catch (error) {
      console.error('[useConversation] Failed to send message:', error);
      
      // Remove the placeholder assistant message on error
      dispatch({ type: 'FINALIZE_STREAM', payload: '[Error generating response]' });
      dispatch({ type: 'SET_LOADING', payload: false });
      dispatch({ type: 'SET_STREAMING', payload: false });
    }
  }, [dispatch, state.conversationId, state.conversationType, state.ocrResults]);

  /**
   * Get all conversations
   */
  const loadMoreConversations = useCallback(async (): Promise<void> => {//TODO: fix this getting duplicate ids
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
   * Toggle Computer Use mode
   */
  const toggleComputerUse = useCallback((): void => {
    if (state.conversationType === "chat") {
      dispatch({ type: 'SET_CONVERSATION_TYPE', payload: "computer_use" })
    } else {
      dispatch({ type: 'SET_CONVERSATION_TYPE', payload: "chat" })
    }
    console.log(state.conversationType)
  }, [dispatch, state.conversationType])

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
    loadMoreConversations,
    renameConversation,
    dispatchOCRCapture,
    deleteOCRResult,
    toggleComputerUse,
  };
}
