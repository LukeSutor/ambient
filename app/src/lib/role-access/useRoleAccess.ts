'use client';

import { useCallback, useEffect } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { useRoleAccessContext } from './RoleAccessProvider';
import type { 
  ConfirmSignUpRequest, 
  SignUpRequest, 
  AuthResponse, 
  SignUpResponse,
  ResendConfirmationResponse,
} from './types';
import {
  invokeVerifyOtp,
  invokeResendConfirmation,
  invokeSignIn,
  invokeSignInWithGoogle,
  invokeSignUp,
  invokeEmitAuthChanged,
  invokeGetCurrentUser,
  invokeIsSetupComplete,
  invokeLogout,
  invokeRefreshToken,
  invokeGetAuthState,
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
  console.log(state.userInfo)

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
    async (email: string, password: string): Promise<AuthResponse> => {
      try {
        const result = await invokeSignIn(email, password);
        
        if (result.session) {
          dispatch({ type: 'SET_LOGGED_IN', payload: true });
        }
        
        // Extract user info from response
        if (result.user) {
          dispatch({ type: 'SET_USER_INFO', payload: {
            id: result.user.id,
            email: result.user.email,
            given_name: result.user.user_metadata?.given_name ?? null,
            family_name: result.user.user_metadata?.family_name ?? null,
            email_verified: result.user.user_metadata?.email_verified ?? null,
            provider: result.user.app_metadata?.provider ?? null,
            created_at: result.user.created_at ?? null,
            providers: result.user.identities?.map((identity) => identity.provider) ?? [],
          }});
        }
        
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

  /**
   * Initiates Google OAuth sign-in flow
   * Opens the authorization URL in the system browser
   * The actual session creation happens via deep link callback
   */
  const signInWithGoogle = useCallback(async (): Promise<void> => {
    try {
      const { url } = await invokeSignInWithGoogle();
      
      // Open the OAuth URL in the system browser
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
      
      // Note: The actual session creation happens when the deep link callback
      // is received in deep_link.rs, which emits 'oauth2-success' or 'oauth2-error'
    } catch (error) {
      console.error('Error during signInWithGoogle:', error);
      throw error;
    }
  }, []);

  const signUp = useCallback(async (request: SignUpRequest): Promise<SignUpResponse> => {
    try {
      return await invokeSignUp(request);
    } catch (error) {
      console.error('Error during signUp:', error);
      throw error;
    }
  }, []);

  const confirmSignUp = useCallback(async (request: ConfirmSignUpRequest): Promise<void> => {
    try {
      await invokeVerifyOtp(request.email, request.confirmation_code);
    } catch (error) {
      console.error('Error during confirmSignUp:', error);
      throw error;
    }
  }, []);

  const resendConfirmationCode = useCallback(async (email: string): Promise<ResendConfirmationResponse> => {
    try {
      return await invokeResendConfirmation(email);
    } catch (error) {
      console.error('Error during resendConfirmationCode:', error);
      throw error;
    }
  }, []);

  const refreshSession = useCallback(async (): Promise<void> => {
    try {
      await invokeRefreshToken();
      // Refresh the auth state after token refresh
      const authState = await invokeGetAuthState();
      dispatch({ type: 'SET_LOGGED_IN', payload: authState.is_authenticated });
      if (authState.user) {
        dispatch({ type: 'SET_USER_INFO', payload: authState.user });
      }
    } catch (error) {
      console.error('Error during refreshSession:', error);
      // If refresh fails, user may need to re-login
      dispatch({ type: 'SET_LOGGED_IN', payload: false });
      dispatch({ type: 'SET_USER_INFO', payload: null });
      throw error;
    }
  }, [dispatch]);

  const signOut = useCallback(async (): Promise<void> => {
    try {
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

      if (user.email?.startsWith('google_')) {
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
    signInWithGoogle,
    signUp,
    confirmSignUp,
    resendConfirmationCode,
    signOut,
    refreshSession,
    getAuthMethod,
    refresh,
  };
}