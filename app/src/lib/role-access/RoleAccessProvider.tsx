'use client';

import React, { createContext, useContext, useReducer, ReactNode, MutableRefObject } from 'react';
import { UserSettings, HudSizeOption, ModelSelection } from '@/types/settings';
import { CognitoUserInfo, RoleAccessState } from './types';
/**
 * Initial state
 */
const initialState: RoleAccessState = {
  isLoggedIn: false,
  isSetupComplete: false,
  isPremiumUser: false,
  userInfo: null,
};

/**
 * Action types
 */
type RoleAccessAction =
  | { type: 'SET_LOGGED_IN'; payload: boolean }
  | { type: 'SET_SETUP_COMPLETE'; payload: boolean }
  | { type: 'SET_PREMIUM_USER'; payload: boolean }
  | { type: 'SET_USER_INFO'; payload: CognitoUserInfo | null };

/**
 * Role access reducer
 */
function roleAccessReducer(state: RoleAccessState, action: RoleAccessAction): RoleAccessState {
  switch (action.type) {
    case 'SET_LOGGED_IN':
      return {
        ...state,
        isLoggedIn: action.payload,
      };

    case 'SET_SETUP_COMPLETE':
      return {
        ...state,
        isSetupComplete: action.payload,
      };

    case 'SET_PREMIUM_USER':
      return {
        ...state,
        isPremiumUser: action.payload,
      };
      
    case 'SET_USER_INFO':
      return {
        ...state,
        userInfo: action.payload,
      };

    default:
      return state;
  }
}

/**
 * Context type
 */
interface RoleAccessContextType {
  state: RoleAccessState;
  dispatch: React.Dispatch<RoleAccessAction>;
}

/**
 * Role Access Context
 */
const RoleAccessContext = createContext<RoleAccessContextType | undefined>(undefined);

/**
 * Role Access Provider Props
 */
interface RoleAccessProviderProps {
  children: ReactNode;
}

/**
 * Role Access Provider Component
 * Provides shared role access state across the application
 */
export function RoleAccessProvider({ children }: RoleAccessProviderProps) {
  const [state, dispatch] = useReducer(roleAccessReducer, initialState);

  return (
    <RoleAccessContext.Provider value={{ state, dispatch }}>
      {children}
    </RoleAccessContext.Provider>
  );
}

/**
 * Hook to access role access context
 * Must be used within a RoleAccessProvider
 */
export function useRoleAccessContext(): RoleAccessContextType {
  const context = useContext(RoleAccessContext);
  
  if (!context) {
    throw new Error('useRoleAccessContext must be used within a RoleAccessProvider');
  }
  
  return context;
}
