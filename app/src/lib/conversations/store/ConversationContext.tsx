'use client';

import React, { createContext, useContext, useReducer, ReactNode } from 'react';
import { ConversationState, initialConversationState } from '../types';
import { conversationReducer, ConversationAction } from './reducer';

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
 * Wraps the application to provide conversation state
 */
export function ConversationProvider({ children }: ConversationProviderProps) {
  const [state, dispatch] = useReducer(conversationReducer, initialConversationState);

  return (
    <ConversationContext.Provider value={{ state, dispatch }}>
      {children}
    </ConversationContext.Provider>
  );
}

/**
 * Hook to access conversation state
 */
export function useConversationState(): ConversationState {
  const context = useContext(ConversationContext);
  
  if (!context) {
    throw new Error('useConversationState must be used within a ConversationProvider');
  }
  
  return context.state;
}

/**
 * Hook to access conversation dispatch
 */
export function useConversationDispatch(): React.Dispatch<ConversationAction> {
  const context = useContext(ConversationContext);
  
  if (!context) {
    throw new Error('useConversationDispatch must be used within a ConversationProvider');
  }
  
  return context.dispatch;
}

/**
 * Hook to access both state and dispatch
 */
export function useConversationContext(): ConversationContextType {
  const context = useContext(ConversationContext);
  
  if (!context) {
    throw new Error('useConversationContext must be used within a ConversationProvider');
  }
  
  return context;
}
