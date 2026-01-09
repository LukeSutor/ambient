'use client';

import React, { createContext, useContext, useReducer, ReactNode, useRef } from 'react';
import { ChatMessage, ConversationState } from './types';
import { Conversation } from '@/types/conversations';
import { MemoryEntry } from '@/types/memory';
import { OcrResponseEvent } from '@/types/events';

/**
 * Initial state for conversations
 */
const initialState: ConversationState = {
  conversationId: null,
  conversationType: 'chat',
  messages: [],
  isStreaming: false,
  isLoading: false,
  streamingContent: '',
  ocrResults: [],
  ocrLoading: false,
  ocrTimeoutRef: { current: null },
  conversations: [],
  conversationPage: 0,
  hasMoreConversations: true,
  initializationRef: { current: false },
};

/**
 * Action types for the conversation reducer
 */
type ConversationAction =
  | { type: 'SET_CONVERSATION_ID'; payload: string | null }
  | { type: 'SET_CONVERSATION_TYPE'; payload: string }
  | { type: 'SET_CONVERSATIONS'; payload: Conversation[] }
  | { type: 'ADD_CONVERSATIONS'; payload: Conversation[] }
  | { type: 'RENAME_CONVERSATION'; payload: { id: string; newName: string } }
  | { type: 'DELETE_CONVERSATION'; payload: { id: string } }
  | { type: 'SET_NO_MORE_CONVERSATIONS' }
  | { type: 'LOAD_CONVERSATION'; payload: Conversation }
  | { type: 'LOAD_MESSAGES'; payload: ChatMessage[] }
  | { type: 'LOAD_REASONING_MESSAGES'; payload: ChatMessage[] }
  | { type: 'ADD_CHAT_MESSAGE'; payload: ChatMessage }
  | { type: 'ADD_REASONING_MESSAGE'; payload: ChatMessage }
  | { type: 'START_USER_MESSAGE'; payload: { id: string; conversationId: string; timestamp: string } }
  | { type: 'FINALIZE_USER_MESSAGE'; payload: { id: string; content: string } }
  | { type: 'START_ASSISTANT_MESSAGE'; payload: { conversationId: string } }
  | { type: 'UPDATE_STREAMING_CONTENT'; payload: string }
  | { type: 'FINALIZE_STREAM'; payload: string }
  | { type: 'ATTACH_MEMORY'; payload: { messageId: string; memory: MemoryEntry } }
  | { type: 'ADD_OCR_RESULT'; payload: OcrResponseEvent }
  | { type: 'DELETE_OCR_RESULT'; payload: number }
  | { type: 'CLEAR_OCR_RESULTS' }
  | { type: 'SET_OCR_TIMEOUT'; payload: ReturnType<typeof setTimeout> | null }
  | { type: 'CLEAR_OCR_TIMEOUT' }
  | { type: 'CLEAR_MESSAGES' }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_STREAMING'; payload: boolean }
  | { type: 'SET_OCR_LOADING'; payload: boolean };

/**
 * Conversation reducer - handles all state updates
 */
function conversationReducer(
  state: ConversationState,
  action: ConversationAction
): ConversationState {
  switch (action.type) {
    case 'SET_CONVERSATION_ID':
      return {
        ...state,
        conversationId: action.payload,
      };

    case 'SET_CONVERSATION_TYPE':
      return {
        ...state,
        conversationType: action.payload,
      }

    case 'RENAME_CONVERSATION':
      return {
        ...state,
        conversations: state.conversations.map((conv) =>
          conv.id === action.payload.id ? { ...conv, name: action.payload.newName } : conv
        ),
      };

    case 'DELETE_CONVERSATION':
      return {
        ...state,
        conversations: state.conversations.filter((conv) => conv.id !== action.payload.id),
      };

    case 'SET_CONVERSATIONS':
      return {
        ...state,
        conversations: action.payload,
      };

    case 'ADD_CONVERSATIONS': {
      // Efficiently deduplicate and sort by updated_at
      //TODO: this needs to be fixed in a more elegant way with realtime conversation list updates, but thats an issue for future me
      const existing = state.conversations;
      const incoming = action.payload;
      const previousLength = existing.length;

      const merged: Conversation[] = [];
      const seenIds = new Set<string>();
      let i = 0;
      let j = 0;

      const compare = (a: Conversation, b: Conversation) => {
      if (a.updated_at === b.updated_at) return 0;
      return a.updated_at > b.updated_at ? -1 : 1;
      };

      while (i < existing.length && j < incoming.length) {
      const currentExisting = existing[i];
      const currentIncoming = incoming[j];

      if (currentExisting.id === currentIncoming.id) {
        const chosen =
        currentExisting.updated_at >= currentIncoming.updated_at
          ? currentExisting
          : currentIncoming;

        if (!seenIds.has(chosen.id)) {
        merged.push(chosen);
        seenIds.add(chosen.id);
        }

        i++;
        j++;
        continue;
      }

      if (compare(currentExisting, currentIncoming) <= 0) {
        if (!seenIds.has(currentExisting.id)) {
        merged.push(currentExisting);
        seenIds.add(currentExisting.id);
        }
        i++;
      } else {
        if (!seenIds.has(currentIncoming.id)) {
        merged.push(currentIncoming);
        seenIds.add(currentIncoming.id);
        }
        j++;
      }
      }

      while (i < existing.length) {
      const current = existing[i];
      if (!seenIds.has(current.id)) {
        merged.push(current);
        seenIds.add(current.id);
      }
      i++;
      }

      while (j < incoming.length) {
      const current = incoming[j];
      if (!seenIds.has(current.id)) {
        merged.push(current);
        seenIds.add(current.id);
      }
      j++;
      }

      return {
      ...state,
      conversations: merged,
      hasMoreConversations: merged.length === previousLength ? false : state.hasMoreConversations,
      };
    }

    case 'SET_NO_MORE_CONVERSATIONS':
      return {
        ...state,
        hasMoreConversations: false,
      };

    case 'LOAD_CONVERSATION':
      return {
        ...state,
        conversationId: action.payload.id,
        conversationType: action.payload.conv_type,
      };

    case 'LOAD_MESSAGES':
      return {
        ...state,
        messages: action.payload,
      };

    case 'ADD_CHAT_MESSAGE':
      return {
        ...state,
        messages: [...state.messages, action.payload],
      };

    case 'START_USER_MESSAGE':
      const newUserMessage: ChatMessage = {
        message: {
          id: action.payload.id,
          conversation_id: action.payload.conversationId,
          role: 'user',
          content: '',
          timestamp: action.payload.timestamp,
        },
        reasoningMessages: [],
        memory: null,
      };
      return {
        ...state,
        messages: [
          ...state.messages,
          newUserMessage,
        ],
      };

    case 'FINALIZE_USER_MESSAGE': {
      // Find user message by ID and update its content
      const updatedMessages = state.messages.map((msg) => {
        if (msg.message.id === action.payload.id && msg.message.role === 'user') {
          return {
            ...msg,
            message: {
              ...msg.message,
              content: action.payload.content,
            },
          };
        }
        return msg;
      });

      return {
        ...state,
        messages: updatedMessages,
      };
    }

    case 'START_ASSISTANT_MESSAGE':
      const newMessage: ChatMessage = {
        message: {
          id: crypto.randomUUID(),
          conversation_id: action.payload.conversationId,
          role: 'assistant',
          content: '',
          timestamp: new Date().toISOString(),
        },
        reasoningMessages: [],
        memory: null,
      };
      return {
        ...state,
        messages: [
          ...state.messages,
          newMessage
        ],
        isStreaming: true,
        streamingContent: '',
      };

    case 'UPDATE_STREAMING_CONTENT': {
      // Find the last assistant message and update its content
      const updatedMessages = [...state.messages];
      const lastAssistantIndex = [...updatedMessages]
        .reverse()
        .findIndex((m) => m.message.role === 'assistant');
      
      if (lastAssistantIndex !== -1) {
        const actualIndex = updatedMessages.length - 1 - lastAssistantIndex;
        updatedMessages[actualIndex] = {
          ...updatedMessages[actualIndex],
          message: {
            ...updatedMessages[actualIndex].message,
            content: action.payload,
          },
        };
      }

      return {
        ...state,
        messages: updatedMessages,
        streamingContent: action.payload,
      };
    }

    case 'FINALIZE_STREAM': {
      // Update the last assistant message with final content
      const finalizedMessages = [...state.messages];
      const lastAssistIdx = [...finalizedMessages]
        .reverse()
        .findIndex((m) => m.message.role === 'assistant');
      
      if (lastAssistIdx !== -1) {
        const actualIdx = finalizedMessages.length - 1 - lastAssistIdx;
        finalizedMessages[actualIdx] = {
          ...finalizedMessages[actualIdx],
          message: {
            ...finalizedMessages[actualIdx].message,
            content: action.payload,
          },
        };
      }

      return {
        ...state,
        messages: finalizedMessages,
        isStreaming: false,
        streamingContent: '',
        isLoading: false,
      };
    }

    case 'ATTACH_MEMORY': {
      // Find user message by ID and attach memory
      const messagesWithMemory = state.messages.map((msg) => {
        if (msg.message.id === action.payload.messageId && msg.message.role === 'user') {
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
    }

    case 'ADD_OCR_RESULT':
      return {
        ...state,
        ocrResults: [...state.ocrResults, action.payload],
      };

    case 'CLEAR_OCR_RESULTS':
      return {
        ...state,
        ocrResults: [],
      };

    case 'DELETE_OCR_RESULT':
      return {
        ...state,
        ocrResults: state.ocrResults.filter((_, idx) => idx !== action.payload),
      };

    case 'SET_OCR_TIMEOUT':
      return {
        ...state,
        ocrTimeoutRef: { current: action.payload },
      };

    case 'CLEAR_OCR_TIMEOUT':
      if (state.ocrTimeoutRef.current) {
        clearTimeout(state.ocrTimeoutRef.current);
      }
      return {
        ...state,
        ocrTimeoutRef: { current: null },
      };

    case 'CLEAR_MESSAGES':
      return {
        ...state,
        messages: [],
        isStreaming: false,
        isLoading: false,
        streamingContent: '',
      };

    case 'SET_LOADING':
      return {
        ...state,
        isLoading: action.payload,
      };

    case 'SET_STREAMING':
      return {
        ...state,
        isStreaming: action.payload,
      };

    case 'SET_OCR_LOADING':
      return {
        ...state,
        ocrLoading: action.payload,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface ConversationContextType {
  state: ConversationState;
  dispatch: React.Dispatch<ConversationAction>;
}

/**
 * Conversation Context
 */
const ConversationContext = createContext<ConversationContextType | undefined>(undefined);

/**
 * Conversation Provider Props
 */
interface ConversationProviderProps {
  children: ReactNode;
}

/**
 * Conversation Provider Component
 * Wraps the application to provide shared conversation state
 */
export function ConversationProvider({ children }: ConversationProviderProps) {
  const [state, dispatch] = useReducer(conversationReducer, initialState);

  return (
    <ConversationContext.Provider value={{ state, dispatch }}>
      {children}
    </ConversationContext.Provider>
  );
}

/**
 * Hook to access conversation context
 * Must be used within a ConversationProvider
 */
export function useConversationContext(): ConversationContextType {
  const context = useContext(ConversationContext);
  
  if (!context) {
    throw new Error('useConversationContext must be used within a ConversationProvider');
  }
  
  return context;
}
