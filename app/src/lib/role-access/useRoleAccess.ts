'use client';

import { useEffect, useCallback, useRef } from 'react';
import { useRoleAccessContext } from './RoleAccessProvider';

export function useRoleAccess(location?: string) {
  const { state, dispatch } = useRoleAccessContext();

  // ============================================================
  // Effects
  // ============================================================
  useEffect(() => {
    if (!location) return;
    
    // Example effect: Log when user login status changes
    console.log('User logged in status changed:', state.isLoggedIn);
  }, [state.isLoggedIn]);

  // ============================================================
  // Modifiers
  // ============================================================
  const setLoggedIn = (value: boolean) => dispatch({ type: 'SET_LOGGED_IN', payload: value });
  const setSetupComplete = (value: boolean) => dispatch({ type: 'SET_SETUP_COMPLETE', payload: value });
  const setPremiumUser = (value: boolean) => dispatch({ type: 'SET_PREMIUM_USER', payload: value });

  return {
    ...state,
    setLoggedIn,
    setSetupComplete,
    setPremiumUser,
  };
}