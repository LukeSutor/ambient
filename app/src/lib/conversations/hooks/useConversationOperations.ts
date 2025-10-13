'use client';

import { useCallback } from 'react';
import { useConversationDispatch } from '../store';
import * as actions from '../store/actions';
import * as api from '../api';
import { createUserMessage, createAssistantMessage } from '../transformers';
import { OcrResponseEvent } from '@/types/events';
import { showErrorToast } from '../utils/notifications';

/**
 * Hook providing conversation operation functions
 */
export function useConversationOperations() {
  const dispatch = useConversationDispatch();

  /**
   * Creates a new conversation
   */
  const createNew = useCallback(async (name?: string): Promise<string | null> => {
    try {
      const conversation = await api.createConversation(name);
      dispatch(actions.setConversationId(conversation.id));
      dispatch(actions.clearMessages());
      return conversation.id;
    } catch (error) {
      console.error('[ConversationOps] Failed to create conversation:', error);
      showErrorToast('Failed to create new conversation');
      return null;
    }
  }, [dispatch]);

  /**
   * Loads messages for the current conversation
   */
  const loadConversationMessages = useCallback(async (conversationId: string): Promise<void> => {
    try {
      const messages = await api.loadMessages(conversationId);
      dispatch(actions.loadMessages(messages));
    } catch (error) {
      console.error('[ConversationOps] Failed to load messages:', error);
      showErrorToast('Failed to load conversation messages');
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
        console.warn('[ConversationOps] Empty message, skipping send');
        return;
      }

      if (!conversationId) {
        throw new Error('No conversation ID provided');
      }

      // Optimistically add user message
      const userMessage = createUserMessage(content);
      dispatch(actions.addUserMessage(userMessage));

      // Start assistant message placeholder
      dispatch(actions.startAssistantMessage());
      dispatch(actions.setLoading(true));
      dispatch(actions.setStreaming(true));

      // Send to backend with the message ID (streaming will be handled by event listeners)
      await api.sendMessage(conversationId, content, ocrResults, userMessage.id);

      // Safety: if no stream events arrive, finalize will happen in stream handler
    } catch (error) {
      console.error('[ConversationOps] Failed to send message:', error);
      
      // Remove the placeholder assistant message on error
      dispatch(actions.finalizeStream('[Error generating response]'));
      dispatch(actions.setLoading(false));
      dispatch(actions.setStreaming(false));
      
      showErrorToast('Failed to send message');
    }
  }, [dispatch]);

  /**
   * Clears all messages in the current conversation
   */
  const clear = useCallback((): void => {
    dispatch(actions.clearMessages());
  }, [dispatch]);

  /**
   * Ensures the llama server is running
   */
  const ensureServer = useCallback(async (): Promise<void> => {
    try {
      await api.ensureLlamaServerRunning();
    } catch (error) {
      console.warn('[ConversationOps] Failed to ensure server running:', error);
      // Non-critical, don't show error to user
    }
  }, []);

  return {
    createNew,
    loadMessages: loadConversationMessages,
    sendMessage,
    clear,
    ensureServer,
  };
}
