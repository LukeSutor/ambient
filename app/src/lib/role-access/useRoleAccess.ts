'use client';

import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useRouter } from 'next/navigation';
import { useRoleAccessContext } from './RoleAccessProvider';
import { SignInResult, CognitoUserInfo, SignUpRequest, ConfirmSignUpRequest, SignUpResult, AuthToken } from './types';
import { set } from 'react-hook-form';
import { emit } from 'process';

async function googleSignOut(): Promise<void> {
  return invoke<void>('google_sign_out');
}

async function logout(): Promise<string> {
  return invoke<string>('logout');
}

async function isAuthenticated(): Promise<boolean> {
  return invoke<boolean>('is_authenticated');
}

async function isSetupComplete(): Promise<boolean> {
  return invoke<boolean>('check_setup_complete');
}

async function emitAuthChanged(): Promise<void> {
  return invoke<void>('emit_auth_changed');
}

// Get current user info
async function getCurrentUserInfo(): Promise<CognitoUserInfo | null> {
  return invoke<CognitoUserInfo | null>('get_current_user');
}

export function useRoleAccess(location?: string) {
  const { state, dispatch } = useRoleAccessContext();
  const router = useRouter();

  // ============================================================
  // Helpers
  // ============================================================

  const fetchInitialInfo = useCallback(async () => {
    try {
      // Auth state
      const loggedIn = await isAuthenticated();
      dispatch({ type: 'SET_LOGGED_IN', payload: loggedIn });
      // Setup state
      const setupComplete = await isSetupComplete();
      dispatch({ type: 'SET_SETUP_COMPLETE', payload: setupComplete });
      // User info if logged in
      if (loggedIn) {
        const userInfo = await getCurrentUserInfo();
        dispatch({ type: 'SET_USER_INFO', payload: userInfo });
      }
    } catch (error) {
      console.error('Error fetching initial info:', error);
    }
  }, []);

  // ============================================================
  // Effects - only use when location is provided
  // ============================================================
  
  if (location) {
    useEffect(() => {
      // Fetch initial auth and setup status
      if (!location) return;
      let unlistenRef: UnlistenFn | null = null;
      (async () => {
        await fetchInitialInfo();

        // Set up auth changed event listener
        const unlisten: UnlistenFn = await listen('auth_changed', async () => {
          try {
            await fetchInitialInfo();
          } catch (error) {
            console.error('Error during auth/setup check on auth_changed event:', error);
          }
        });
        unlistenRef = unlisten;
      })();

      return () => {
        // Clean up event listener on unmount
        if (unlistenRef) {
          unlistenRef();
        }
      };
    }, []);
  
    // Redirect based on location and login status
    useEffect(() => {
      if (!location) return;
  
      if (state.isLoggedIn) {
        // Redirect to location if logged in
        router.push(location);
      } else {
        // Redirect to login if not logged in
        router.push(location+'/signin');
      }
      
      // Example effect: Log when user login status changes
      console.log('User logged in status changed:', state.isLoggedIn);
    }, [state.isLoggedIn]);
  
    // Redirect based on setup completion
    useEffect(() => {
      if (!location) return;
  
      // Example effect: Log when setup completion status changes
      console.log('User setup completion status changed:', state.isSetupComplete);
    }, [state.isSetupComplete]);
  }

  // ============================================================
  // Auth Actions
  // ============================================================

  // Sign in with username and password using Cognito
  const signIn = useCallback(async (username: string, password: string): Promise<SignInResult> => {
    try {
      const result = await invoke<SignInResult>('cognito_sign_in', {
      username,
      password,
      });
      dispatch({ type: 'SET_LOGGED_IN', payload: true });
      dispatch({ type: 'SET_USER_INFO', payload: result.user_info });
      await emitAuthChanged();
      return result;
    } catch (error) {
      console.error('Error during signIn:', error);
      throw error;
    }
  }, []);

  // Google sign in
  const googleSignIn = useCallback(async (): Promise<SignInResult> => {
    try {
      const result = await invoke<SignInResult>('google_sign_in');
      dispatch({ type: 'SET_LOGGED_IN', payload: true });
      await emitAuthChanged();
      return result;
    } catch (error) {
      console.error('Error during googleSignIn:', error);
      throw error;
    }
  }, []);

  // Sign up a new user with Cognito
  const signUp = useCallback(async (request: SignUpRequest): Promise<SignUpResult> => {
    try {
      return invoke<SignUpResult>('cognito_sign_up', {
        username: request.username,
        password: request.password,
        email: request.email,
        givenName: request.given_name,
        familyName: request.family_name,
      });
    } catch (error) {
      console.error('Error during signUp:', error);
      throw error;
    }
  }, []);

  // Confirm user sign up with confirmation code
  const confirmSignUp = useCallback(async (request: ConfirmSignUpRequest): Promise<void> => {
    try {
      await invoke('cognito_confirm_sign_up', {
        username: request.username,
        confirmationCode: request.confirmation_code,
        session: request.session,
      });
    } catch (error) {
      console.error('Error during confirmSignUp:', error);
      throw error;
    }
  }, []);

  // Resend confirmation code for user verification
  const resendConfirmationCode = useCallback(async (username: string): Promise<SignUpResult> => {
    try {
      return invoke<SignUpResult>('cognito_resend_confirmation_code', {
        username,
      });
    } catch (error) {
      console.error('Error during resendConfirmationCode:', error);
      throw error;
    }
  }, []);

  // Log out user (handles both Cognito and Google sign out)
  const signOut = useCallback(async (): Promise<void> => {
    try {
      // Perform google sign out then cognito logout
      await googleSignOut();
      await logout();
      dispatch({ type: 'SET_LOGGED_IN', payload: false });
      await emitAuthChanged();
      console.log('User signed out successfully');
    } catch (error) {
      console.error('Error during signOut:', error);
      throw error;
    }
  }, []);

  // Get authentication method
  const getAuthMethod = useCallback(async (): Promise<'google' | 'cognito' | 'unknown'> => {
    try {
      const user = await getCurrentUserInfo();
      if (!user) return 'unknown';

      // Google OAuth users typically have email as username or specific patterns
      // This is a heuristic and may need adjustment based on actual data patterns
      if (user.username && user.username.startsWith('google_')) {
        return 'google';
      }

      // Default to cognito for regular usernames
      return 'cognito';
    } catch (error) {
      console.error('Failed to determine authentication method:', error);
      return 'unknown';
    }
  }, []);

  return {
    ...state,
    signIn,
    googleSignIn,
    signUp,
    confirmSignUp,
    resendConfirmationCode,
    signOut,
    getAuthMethod,
    isSetupComplete,
  };
}