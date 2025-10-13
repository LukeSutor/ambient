// Main exports for conversation management library
export { ConversationProvider } from './store';
export { useConversationManager } from './hooks';

// Type exports
export type { 
  ChatMessage, 
  Conversation, 
  ConversationState, 
  MessageContext,
  MessageRole 
} from './types';

// Additional hooks for advanced usage
export { 
  useConversation, 
  useConversationOperations,
  useConversationEvents 
} from './hooks';

// Utility exports
export { 
  extractThinkingContent,
  transformBackendMessage,
  createUserMessage,
  createAssistantMessage 
} from './transformers';
