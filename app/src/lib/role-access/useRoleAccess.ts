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
    const setupPath = `${normalizedLocation}/setup`;
    const isSetupRoute = pathname === setupPath || pathname.startsWith(`${setupPath}/`);
    const isWithinBase =
      pathname === normalizedLocation || pathname.startsWith(`${normalizedLocation}/`);

    // Go to sign in if online and not logged in
    //TODO: Fix logic for offline mode
    if (!state.isLoggedIn && isWithinBase && !isAuthRoute) {
      router.replace(signInPath);
      return;
    }

    // Go to setup if logged in but setup not complete
    if (!state.isSetupComplete && isWithinBase && !isAuthRoute && !isSetupRoute) {
      router.replace(setupPath);
      return;
    }

    // Prevent going to auth routes if not online or already logged in
    if ((state.isLoggedIn) && isAuthRoute) {
      router.replace(normalizedLocation);
    }

    // Prevent going to setup routes if setup is complete
    if (state.isSetupComplete && isSetupRoute) {
      router.replace(normalizedLocation);
    }
  }, [normalizedLocation, pathname, router, state.isHydrated, state.isOnline, state.isLoggedIn, state.isSetupComplete]);

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