import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { ChatStreamEvent } from '@/types/events';
import { extractThinkingContent } from '../transformers';
import { ConversationAction } from '../store/reducer';
import * as actions from '../store/actions';

/**
 * Sets up listener for chat stream events
 * @param conversationId - Current conversation ID to filter events
 * @param dispatch - Reducer dispatch function
 * @param messagesEndRef - Ref to scroll target (optional)
 * @returns Unlisten function for cleanup
 */
export async function setupStreamListener(
  conversationId: string | null,
  dispatch: React.Dispatch<ConversationAction>,
  messagesEndRef?: React.RefObject<HTMLDivElement | null>
): Promise<UnlistenFn> {
  let streamContent = '';

  const unlisten = await listen<ChatStreamEvent>('chat_stream', (event) => {
    const { delta, full_response, is_finished, conv_id } = event.payload;

    // Filter by conversation ID
    if (conv_id !== conversationId) {
      return;
    }

    if (is_finished) {
      // Stream is complete
      const finalText = extractThinkingContent(full_response ?? streamContent);
      dispatch(actions.finalizeStream(finalText));
      streamContent = '';
      return;
    }

    if (delta) {
      // Accumulate stream content
      streamContent += delta;
      const cleanContent = extractThinkingContent(streamContent);
      dispatch(actions.updateStreamingContent(cleanContent));

      // Auto-scroll to bottom
      if (messagesEndRef?.current) {
        queueMicrotask(() => {
          messagesEndRef.current?.scrollIntoView({ 
            behavior: 'smooth', 
            block: 'end' 
          });
        });
      }
    }
  });

  return unlisten;
}
