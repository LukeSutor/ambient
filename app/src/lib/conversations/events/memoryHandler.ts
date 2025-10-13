import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { MemoryExtractedEvent } from '@/types/events';
import { ConversationAction } from '../store/reducer';
import * as actions from '../store/actions';

/**
 * Sets up listener for memory extraction events
 * @param dispatch - Reducer dispatch function
 * @returns Unlisten function for cleanup
 */
export async function setupMemoryListener(
  dispatch: React.Dispatch<ConversationAction>
): Promise<UnlistenFn> {
  const unlisten = await listen<MemoryExtractedEvent>('memory_extracted', (event) => {
    const { memory } = event.payload;
    
    console.log('[MemoryHandler] Memory extracted:', memory);

    // Attach memory to the message with matching message_id
    if (memory.message_id) {
      dispatch(actions.attachMemory(memory.message_id, memory));
    }
  });

  return unlisten;
}
