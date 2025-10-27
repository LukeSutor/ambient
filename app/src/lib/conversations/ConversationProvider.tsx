'use client';

import React, { createContext, useContext, useReducer, ReactNode, useRef } from 'react';
import { ChatMessage, ConversationState } from './types';
import { Conversation } from '@/types/conversations';
import { MemoryEntry } from '@/types/memory';

/**
 * Initial state for conversations
 */
const initialState: ConversationState = {
  conversationId: null,
  messages: [],
  isStreaming: false,
  isLoading: false,
  streamingContent: '',
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
  | { type: 'SET_CONVERSATIONS'; payload: Conversation[] }
  | { type: 'ADD_CONVERSATIONS'; payload: Conversation[] }
  | { type: 'RENAME_CONVERSATION'; payload: { id: string; newName: string } }
  | { type: 'DELETE_CONVERSATION'; payload: { id: string } }
  | { type: 'SET_NO_MORE_CONVERSATIONS' }
  | { type: 'LOAD_MESSAGES'; payload: ChatMessage[] }
  | { type: 'ADD_USER_MESSAGE'; payload: ChatMessage }
  | { type: 'START_USER_MESSAGE'; payload: { id: string; timestamp: string } }
  | { type: 'FINALIZE_USER_MESSAGE'; payload: { id: string; content: string } }
  | { type: 'START_ASSISTANT_MESSAGE' }
  | { type: 'UPDATE_STREAMING_CONTENT'; payload: string }
  | { type: 'FINALIZE_STREAM'; payload: string }
  | { type: 'ATTACH_MEMORY'; payload: { messageId: string; memory: MemoryEntry } }
  | { type: 'CLEAR_MESSAGES' }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'SET_STREAMING'; payload: boolean };

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

    case 'ADD_CONVERSATIONS':
      return {
        ...state,
        conversations: [...state.conversations, ...action.payload],
      };

    case 'SET_NO_MORE_CONVERSATIONS':
      return {
        ...state,
        hasMoreConversations: false,
      };

    case 'LOAD_MESSAGES':
      return {
        ...state,
        messages: action.payload,
      };

    case 'ADD_USER_MESSAGE':
      return {
        ...state,
        messages: [...state.messages, action.payload],
      };

    case 'START_USER_MESSAGE':
      return {
        ...state,
        messages: [
          ...state.messages,
          {
            id: action.payload.id,
            role: 'user',
            content: '',
            memory: null,
            timestamp: action.payload.timestamp,
          },
        ],
      };

    case 'FINALIZE_USER_MESSAGE': {
      // Find user message by ID and update its content
      const updatedMessages = state.messages.map((msg) => {
        if (msg.id === action.payload.id && msg.role === 'user') {
          return {
            ...msg,
            content: action.payload.content,
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

    case 'UPDATE_STREAMING_CONTENT': {
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
    }

    case 'FINALIZE_STREAM': {
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
    }

    case 'ATTACH_MEMORY': {
      // Find user message by ID and attach memory
      const messagesWithMemory = state.messages.map((msg) => {
        if (msg.id === action.payload.messageId && msg.role === 'user') {
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
