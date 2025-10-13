import { MemoryEntry } from '@/types/memory';
import { OcrResponseEvent } from '@/types/events';

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
  created_at: string;
  updated_at: string;
  message_count: number;
}

/**
 * Conversation state for the reducer
 */
export interface ConversationState {
  conversationId: string | null;
  messages: ChatMessage[];
  isStreaming: boolean;
  isLoading: boolean;
  streamingContent: string;
}

/**
 * Context for sending messages (OCR results, etc.)
 */
export interface MessageContext {
  ocrResults: OcrResponseEvent[];
}

/**
 * Initial state for the conversation reducer
 */
export const initialConversationState: ConversationState = {
  conversationId: null,
  messages: [],
  isStreaming: false,
  isLoading: false,
  streamingContent: '',
};
