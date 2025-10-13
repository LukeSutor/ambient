import { ChatMessage } from '../types';
import { ConversationActionType } from './reducer';
import { MemoryEntry } from '@/types/memory';

/**
 * Action creators for conversation operations
 */

export const setConversationId = (conversationId: string | null) => ({
  type: ConversationActionType.SET_CONVERSATION_ID as const,
  payload: conversationId,
});

export const loadMessages = (messages: ChatMessage[]) => ({
  type: ConversationActionType.LOAD_MESSAGES as const,
  payload: messages,
});

export const addUserMessage = (message: ChatMessage) => ({
  type: ConversationActionType.ADD_USER_MESSAGE as const,
  payload: message,
});

export const startAssistantMessage = () => ({
  type: ConversationActionType.START_ASSISTANT_MESSAGE as const,
});

export const updateStreamingContent = (content: string) => ({
  type: ConversationActionType.UPDATE_STREAMING_CONTENT as const,
  payload: content,
});

export const finalizeStream = (finalContent: string) => ({
  type: ConversationActionType.FINALIZE_STREAM as const,
  payload: finalContent,
});

export const attachMemory = (messageId: string, memory: MemoryEntry) => ({
  type: ConversationActionType.ATTACH_MEMORY as const,
  payload: { messageId, memory },
});

export const clearMessages = () => ({
  type: ConversationActionType.CLEAR_MESSAGES as const,
});

export const setLoading = (isLoading: boolean) => ({
  type: ConversationActionType.SET_LOADING as const,
  payload: isLoading,
});

export const setStreaming = (isStreaming: boolean) => ({
  type: ConversationActionType.SET_STREAMING as const,
  payload: isStreaming,
});
