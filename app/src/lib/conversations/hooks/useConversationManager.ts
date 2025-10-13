'use client';

import { useEffect, useRef } from 'react';
import { useConversation } from './useConversation';
import { useConversationOperations } from './useConversationOperations';
import { useConversationEvents } from './useConversationEvents';

/**
 * Main conversation manager hook
 * Composes all conversation functionality into a single, easy-to-use hook
 * 
 * This is the primary hook that components should use for conversation management
 */
export function useConversationManager(messagesEndRef?: React.RefObject<HTMLDivElement | null>) {
  const conversation = useConversation();
  const operations = useConversationOperations();
  const initializedRef = useRef(false);

  // Set up event listeners
  useConversationEvents(messagesEndRef);

  // Initialize: ensure server running and create initial conversation
  useEffect(() => {
    if (initializedRef.current) return;
    initializedRef.current = true;

    const initialize = async () => {
      console.log('[ConversationManager] Initializing...');
      
      // Ensure llama server is running
      await operations.ensureServer();

      // Create initial conversation if none exists
      if (!conversation.conversationId) {
        const newId = await operations.createNew();
        if (newId) {
          console.log('[ConversationManager] Created initial conversation:', newId);
        }
      }
    };

    initialize();
  }, []); // Only run once on mount

  return {
    // State
    ...conversation,
    
    // Operations
    ...operations,
  };
}
