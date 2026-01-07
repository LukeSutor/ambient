import { MemoryEntry } from '@/types/memory';
import { OcrResponseEvent } from '@/types/events';
import { RefObject } from 'react';
import { Conversation, Role } from '@/types/conversations';

/**
 * Chat message structure with optional memory attachment
 */
export interface ChatMessage {
  id: string;
  role: Role;
  content: string;
  memory: MemoryEntry | null;
  timestamp: string;
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
