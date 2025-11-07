'use client';

import { useCallback, useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { useRoleAccessContext } from './RoleAccessProvider';
import type { ConfirmSignUpRequest, SignInResult, SignUpRequest, SignUpResult } from './types';
import {
  invokeCognitoConfirmSignUp,
  invokeCognitoResendConfirmationCode,
  invokeCognitoSignIn,
  invokeCognitoSignUp,
  invokeEmitAuthChanged,
  invokeGetCurrentUser,
  invokeIsSetupComplete,
  invokeGoogleSignIn,
  invokeGoogleSignOut,
  invokeLogout,
} from './commands';

function normalizeBasePath(base?: string): string | null {
  if (!base) {
    return null;
  }

  return base.endsWith('/') ? base.slice(0, -1) : base;
}

export function useRoleAccess(location?: string) {
  const { state, dispatch, refresh } = useRoleAccessContext();
  const router = useRouter();
  const pathname = usePathname();
  const normalizedLocation = normalizeBasePath(location ?? undefined);

  useEffect(() => {
    if (!normalizedLocation || !state.isHydrated || !pathname) {
      return;
    }

    const signInPath = `${normalizedLocation}/signin`;
    const signUpPath = `${normalizedLocation}/signup`;
    const isAuthRoute =
      pathname === signInPath ||
      pathname === signUpPath ||
      pathname.startsWith(`${signInPath}/`) ||
      pathname.startsWith(`${signUpPath}/`);
    const isWithinBase =
      pathname === normalizedLocation || pathname.startsWith(`${normalizedLocation}/`);

    let redirectTimer: number | undefined;

    if (!state.isLoggedIn && isWithinBase && !isAuthRoute) {
      redirectTimer = window.setTimeout(() => {
        router.replace(signInPath);
      }, 200);
    }

    if (state.isLoggedIn && isAuthRoute) {
      router.replace(normalizedLocation);
    }

    return () => {
      if (redirectTimer) {
        window.clearTimeout(redirectTimer);
      }
    };
  }, [normalizedLocation, pathname, router, state.isHydrated, state.isLoggedIn]);

  const signIn = useCallback(
    async (username: string, password: string): Promise<SignInResult> => {
      try {
        const result = await invokeCognitoSignIn(username, password);
        dispatch({ type: 'SET_LOGGED_IN', payload: true });
        dispatch({ type: 'SET_USER_INFO', payload: result.user_info });
        const setupComplete = await invokeIsSetupComplete();
        dispatch({ type: 'SET_SETUP_COMPLETE', payload: setupComplete });
        await invokeEmitAuthChanged();
        return result;
      } catch (error) {
        console.error('Error during signIn:', error);
        throw error;
      }
    },
  [dispatch],
  );

  const googleSignIn = useCallback(async (): Promise<SignInResult> => {
    try {
      const result = await invokeGoogleSignIn();
      dispatch({ type: 'SET_LOGGED_IN', payload: true });
      if (result.user_info) {
        dispatch({ type: 'SET_USER_INFO', payload: result.user_info });
      }
      const setupComplete = await invokeIsSetupComplete();
      dispatch({ type: 'SET_SETUP_COMPLETE', payload: setupComplete });
      await invokeEmitAuthChanged();
      return result;
    } catch (error) {
      console.error('Error during googleSignIn:', error);
      throw error;
    }
  }, [dispatch]);

  const signUp = useCallback(async (request: SignUpRequest): Promise<SignUpResult> => {
    try {
      return await invokeCognitoSignUp(request);
    } catch (error) {
      console.error('Error during signUp:', error);
      throw error;
    }
  }, []);

  const confirmSignUp = useCallback(async (request: ConfirmSignUpRequest): Promise<void> => {
    try {
      await invokeCognitoConfirmSignUp(request);
    } catch (error) {
      console.error('Error during confirmSignUp:', error);
      throw error;
    }
  }, []);

  const resendConfirmationCode = useCallback(async (username: string): Promise<SignUpResult> => {
    try {
      return await invokeCognitoResendConfirmationCode(username);
    } catch (error) {
      console.error('Error during resendConfirmationCode:', error);
      throw error;
    }
  }, []);

  const signOut = useCallback(async (): Promise<void> => {
    try {
      await invokeGoogleSignOut();
      await invokeLogout();
      dispatch({ type: 'SET_LOGGED_IN', payload: false });
      dispatch({ type: 'SET_USER_INFO', payload: null });
      dispatch({ type: 'SET_SETUP_COMPLETE', payload: false });
      await invokeEmitAuthChanged();
    } catch (error) {
      console.error('Error during signOut:', error);
      throw error;
    }
  }, [dispatch]);

  const getAuthMethod = useCallback(async (): Promise<'google' | 'cognito' | 'unknown'> => {
    try {
      const user = await invokeGetCurrentUser();
      if (!user) {
        return 'unknown';
      }

      if (user.username?.startsWith('google_')) {
        return 'google';
      }

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
    refresh,
  };
}