import { ChatMessage, ConversationState } from '../types';
import { MemoryEntry } from '@/types/memory';

/**
 * Action types for the conversation reducer
 */
export enum ConversationActionType {
  SET_CONVERSATION_ID = 'SET_CONVERSATION_ID',
  LOAD_MESSAGES = 'LOAD_MESSAGES',
  ADD_USER_MESSAGE = 'ADD_USER_MESSAGE',
  START_ASSISTANT_MESSAGE = 'START_ASSISTANT_MESSAGE',
  UPDATE_STREAMING_CONTENT = 'UPDATE_STREAMING_CONTENT',
  FINALIZE_STREAM = 'FINALIZE_STREAM',
  ATTACH_MEMORY = 'ATTACH_MEMORY',
  CLEAR_MESSAGES = 'CLEAR_MESSAGES',
  SET_LOADING = 'SET_LOADING',
  SET_STREAMING = 'SET_STREAMING',
}

/**
 * Action union type
 */
export type ConversationAction =
  | { type: ConversationActionType.SET_CONVERSATION_ID; payload: string | null }
  | { type: ConversationActionType.LOAD_MESSAGES; payload: ChatMessage[] }
  | { type: ConversationActionType.ADD_USER_MESSAGE; payload: ChatMessage }
  | { type: ConversationActionType.START_ASSISTANT_MESSAGE }
  | { type: ConversationActionType.UPDATE_STREAMING_CONTENT; payload: string }
  | { type: ConversationActionType.FINALIZE_STREAM; payload: string }
  | { type: ConversationActionType.ATTACH_MEMORY; payload: { messageId: string; memory: MemoryEntry } }
  | { type: ConversationActionType.CLEAR_MESSAGES }
  | { type: ConversationActionType.SET_LOADING; payload: boolean }
  | { type: ConversationActionType.SET_STREAMING; payload: boolean };

/**
 * Conversation reducer
 */
export function conversationReducer(
  state: ConversationState,
  action: ConversationAction
): ConversationState {
  switch (action.type) {
    case ConversationActionType.SET_CONVERSATION_ID:
      return {
        ...state,
        conversationId: action.payload,
      };

    case ConversationActionType.LOAD_MESSAGES:
      return {
        ...state,
        messages: action.payload,
      };

    case ConversationActionType.ADD_USER_MESSAGE:
      return {
        ...state,
        messages: [...state.messages, action.payload],
      };

    case ConversationActionType.START_ASSISTANT_MESSAGE:
      return {
        ...state,
        messages: [
          ...state.messages,
          {
            id: crypto.randomUUID(),
            role: 'assistant',
            content: '',
            memory: null,
            timestamp: new Date().toISOString(),
          },
        ],
        isStreaming: true,
        streamingContent: '',
      };

    case ConversationActionType.UPDATE_STREAMING_CONTENT:
      // Find the last assistant message and update its content
      const updatedMessages = [...state.messages];
      const lastAssistantIndex = [...updatedMessages]
        .reverse()
        .findIndex((m) => m.role === 'assistant');
      
      if (lastAssistantIndex !== -1) {
        const actualIndex = updatedMessages.length - 1 - lastAssistantIndex;
        updatedMessages[actualIndex] = {
          ...updatedMessages[actualIndex],
          content: action.payload,
        };
      }

      return {
        ...state,
        messages: updatedMessages,
        streamingContent: action.payload,
      };

    case ConversationActionType.FINALIZE_STREAM:
      // Update the last assistant message with final content
      const finalizedMessages = [...state.messages];
      const lastAssistIdx = [...finalizedMessages]
        .reverse()
        .findIndex((m) => m.role === 'assistant');
      
      if (lastAssistIdx !== -1) {
        const actualIdx = finalizedMessages.length - 1 - lastAssistIdx;
        finalizedMessages[actualIdx] = {
          ...finalizedMessages[actualIdx],
          content: action.payload,
        };
      }

      return {
        ...state,
        messages: finalizedMessages,
        isStreaming: false,
        streamingContent: '',
        isLoading: false,
      };

    case ConversationActionType.ATTACH_MEMORY:
      // Find user message by ID and attach memory
      console.log("ATTACH_MEMORY action received");
      console.log("state.messages:", state.messages);
      console.log("action.payload:", action.payload);
      const messagesWithMemory = state.messages.map((msg) => {
        console.log(msg.id + " " + action.payload.messageId);
        if (msg.id === action.payload.messageId && msg.role === 'user') {
            console.log("msg:", msg);
          return {
            ...msg,
            memory: action.payload.memory,
          };
        }
        return msg;
      });

      return {
        ...state,
        messages: messagesWithMemory,
      };

    case ConversationActionType.CLEAR_MESSAGES:
      return {
        ...state,
        messages: [],
        isStreaming: false,
        isLoading: false,
        streamingContent: '',
      };

    case ConversationActionType.SET_LOADING:
      return {
        ...state,
        isLoading: action.payload,
      };

    case ConversationActionType.SET_STREAMING:
      return {
        ...state,
        isStreaming: action.payload,
      };

    default:
      return state;
  }
}
