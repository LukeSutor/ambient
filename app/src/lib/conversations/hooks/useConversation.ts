'use client';

import { useConversationState } from '../store';

/**
 * Hook to access conversation state
 * Provides read-only access to current conversation data
 */
export function useConversation() {
  const state = useConversationState();

  return {
    conversationId: state.conversationId,
    messages: state.messages,
    isStreaming: state.isStreaming,
    isLoading: state.isLoading,
    streamingContent: state.streamingContent,
  };
}
