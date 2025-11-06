'use client';

import { useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useRoleAccessContext } from './RoleAccessProvider';
import { SignInResult, CognitoUserInfo, SignUpRequest, ConfirmSignUpRequest, SignUpResult, AuthToken } from './types';

async function googleSignOut(): Promise<void> {
  return invoke<void>('google_sign_out');
}

async function logout(): Promise<string> {
  return invoke<string>('logout');
}

export function useRoleAccess(location?: string) {
  const { state, dispatch } = useRoleAccessContext();

  // ============================================================
  // Effects
  // ============================================================

  // Redirect based on location and login status
  useEffect(() => {
    if (!location) return;
    
    // Example effect: Log when user login status changes
    console.log('User logged in status changed:', state.isLoggedIn);
  }, [state.isLoggedIn]);

  // Redirect based on setup completion
  useEffect(() => {
    if (!location) return;

    // Example effect: Log when setup completion status changes
    console.log('User setup completion status changed:', state.isSetupComplete);
  }, [state.isSetupComplete]);

  // ============================================================
  // Auth Actions
  // ============================================================

  // Sign in with username and password using Cognito
  const signIn = useCallback(async (username: string, password: string): Promise<SignInResult> => {
    try {
      return await invoke<SignInResult>('cognito_sign_in', {
      username,
      password,
      });
    } catch (error) {
      console.error('Error during signIn:', error);
      throw error;
    }
  }, []);

  // Google sign in
  const googleSignIn = useCallback(async (): Promise<SignInResult> => {
    try {
      const result = await invoke<SignInResult>('google_sign_in');
      setLoggedIn(true);
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
      setLoggedIn(false);
      console.log('User signed out successfully');
    } catch (error) {
      console.error('Error during signOut:', error);
      throw error;
    }
  }, []);

  // Check if user is authenticated
  const isAuthenticated = useCallback(async (): Promise<boolean> => {
    try {
      return await invoke<boolean>('is_authenticated');
    } catch (error) {
      console.error('Error during isAuthenticated:', error);
      throw error;
    }
  }, []);

  // Get current user info
  const getCurrentUserInfo = useCallback(async (): Promise<CognitoUserInfo | null> => {
    try {
      return await invoke<CognitoUserInfo | null>('get_current_user_info');
    } catch (error) {
      console.error('Error during getCurrentUserInfo:', error);
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

  // ============================================================
  // Setup actions
  // ============================================================

  // Check if setup is complete
  const isSetupComplete = useCallback(async (): Promise<boolean> => {
    try {
      return await invoke<boolean>('check_setup_complete');
    } catch (error) {
      console.error('Error during isSetupComplete:', error);
      throw error;
    }
  }, []);

  // ============================================================
  // Modifiers
  // ============================================================
  const setLoggedIn = (value: boolean) => dispatch({ type: 'SET_LOGGED_IN', payload: value });
  const setSetupComplete = (value: boolean) => dispatch({ type: 'SET_SETUP_COMPLETE', payload: value });
  const setPremiumUser = (value: boolean) => dispatch({ type: 'SET_PREMIUM_USER', payload: value });

  return {
    ...state,
    signIn,
    googleSignIn,
    signUp,
    confirmSignUp,
    resendConfirmationCode,
    signOut,
    isAuthenticated,
    getCurrentUserInfo,
    getAuthMethod,
    isSetupComplete,
    setLoggedIn,
    setSetupComplete,
    setPremiumUser,
  };
}