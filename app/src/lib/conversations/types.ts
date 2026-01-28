import { type Conversation, type Message, Role } from "@/types/conversations";
import { type AttachmentData, OcrResponseEvent } from "@/types/events";
import type { MemoryEntry } from "@/types/memory";
import type { RefObject } from "react";

/**
 * Chat message structure with optional memory attachment
 */
export interface ChatMessage {
  message: Message;
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
  ocrLoading: boolean;
  ocrTimeoutRef: RefObject<ReturnType<typeof setTimeout> | null>;
  conversations: Conversation[];
  conversationPage: number;
  hasMoreConversations: boolean;
  initializationRef: RefObject<boolean>;
}
