/**
 * Conversation Management Library
 * Simplified exports for easy consumption
 */

// Provider for shared conversation state
export { ConversationProvider } from './ConversationProvider';

// Main hook for conversation functionality
export { useConversation } from './useConversation';

// Type exports
export type { 
  ChatMessage, 
  Conversation, 
  ConversationState, 
  MessageRole 
} from './types';
