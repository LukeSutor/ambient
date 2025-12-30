import { MemoryEntry } from '@/types/memory';
import { OcrResponseEvent } from '@/types/events';
import { RefObject } from 'react';

/**
 * Message role type
 */
export type MessageRole = 'user' | 'assistant';

/**
 * Chat message structure with optional memory attachment
 */
export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  memory: MemoryEntry | null;
  timestamp: string;
}

/**
 * Conversation metadata
 */
export interface Conversation {
  id: string;
  name: string;
  conv_type: string;
  created_at: string;
  updated_at: string;
  message_count: number;
}

/**
 * Conversation state
 */
export interface ConversationState {
  conversationId: string | null;
  conversationType: string;
  messages: ChatMessage[];
  isStreaming: boolean;
  isLoading: boolean;
  streamingContent: string;
  ocrResults: OcrResponseEvent[];
  ocrLoading: boolean;
  ocrTimeoutRef: RefObject<ReturnType<typeof setTimeout> | null>;
  conversations: Conversation[];
  conversationPage: number;
  hasMoreConversations: boolean;
  initializationRef: RefObject<boolean>;
}
