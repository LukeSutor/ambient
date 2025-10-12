'use client';

import { useEffect, useRef } from 'react';
import { useConversation } from './useConversation';
import { initializeConversationEvents, CleanupFn } from '../events';
import { useConversationDispatch } from '../store';

/**
 * Hook to set up conversation event listeners
 * Automatically subscribes/unsubscribes based on conversation ID
 */
export function useConversationEvents(messagesEndRef?: React.RefObject<HTMLDivElement | null>) {
  const { conversationId } = useConversation();
  const dispatch = useConversationDispatch();
  const cleanupRef = useRef<CleanupFn | null>(null);

  useEffect(() => {
    let isMounted = true;

    const setupEvents = async () => {
      // Clean up previous listeners
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }

      // Set up new listeners if we have a conversation
      if (conversationId && isMounted) {
        try {
          const cleanup = await initializeConversationEvents(
            conversationId,
            dispatch,
            messagesEndRef
          );
          
          if (isMounted) {
            cleanupRef.current = cleanup;
          } else {
            // Component unmounted during async operation
            cleanup();
          }
        } catch (error) {
          console.error('[useConversationEvents] Failed to setup events:', error);
        }
      }
    };

    setupEvents();

    // Cleanup on unmount or conversationId change
    return () => {
      isMounted = false;
      if (cleanupRef.current) {
        cleanupRef.current();
        cleanupRef.current = null;
      }
    };
  }, [conversationId, dispatch, messagesEndRef]);
}
