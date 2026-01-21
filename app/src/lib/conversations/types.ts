import { MemoryEntry } from '@/types/memory';
import { AttachmentData, OcrResponseEvent } from '@/types/events';
import { RefObject } from 'react';
import { Conversation, Message, Role } from '@/types/conversations';

/**
 * Chat message structure with optional memory attachment
 */
export interface ChatMessage {
  message: Message;
  reasoningMessages: ChatMessage[];
  memory: MemoryEntry | null;
}

/**
 * Conversation state
 */
export interface ConversationState {
  conversationId: string | null;
  conversationName: string;
  conversationType: string;
  messages: ChatMessage[];
  attachmentData: AttachmentData[];
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
