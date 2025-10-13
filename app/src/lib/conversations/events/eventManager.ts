import { UnlistenFn } from '@tauri-apps/api/event';
import { ConversationAction } from '../store/reducer';
import { setupStreamListener } from './streamHandler';
import { setupMemoryListener } from './memoryHandler';

/**
 * Cleanup function type
 */
export type CleanupFn = () => void;

/**
 * Initializes all conversation-related event listeners
 * @param conversationId - Current conversation ID
 * @param dispatch - Reducer dispatch function
 * @param messagesEndRef - Optional ref for auto-scrolling
 * @returns Cleanup function to remove all listeners
 */
export async function initializeConversationEvents(
  conversationId: string | null,
  dispatch: React.Dispatch<ConversationAction>,
  messagesEndRef?: React.RefObject<HTMLDivElement | null>
): Promise<CleanupFn> {
  const unlisteners: UnlistenFn[] = [];

  try {
    // Set up stream listener
    const streamUnlisten = await setupStreamListener(
      conversationId,
      dispatch,
      messagesEndRef
    );
    unlisteners.push(streamUnlisten);

    // Set up memory listener
    const memoryUnlisten = await setupMemoryListener(dispatch);
    unlisteners.push(memoryUnlisten);

    console.log('[EventManager] Conversation events initialized');
  } catch (error) {
    console.error('[EventManager] Failed to initialize events:', error);
  }

  // Return cleanup function
  return () => {
    unlisteners.forEach((unlisten) => {
      try {
        unlisten();
      } catch (error) {
        console.error('[EventManager] Error during cleanup:', error);
      }
    });
    console.log('[EventManager] Conversation events cleaned up');
  };
}
