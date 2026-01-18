import { invoke } from '@tauri-apps/api/core';
import type {
  UserInfo,
  SignUpRequest,
  AuthResponse,
  SignUpResponse,
  AuthState,
  RefreshTokenResponse,
  ResendConfirmationResponse,
  VerifyOtpResponse,
  OAuthUrlResponse,
  FullAuthState,
  AuthErrorResponse,
} from './types';

// ============================================================================
// Error Handling Helpers
// ============================================================================

/**
 * Parse a structured auth error from a string response
 */
export function parseAuthError(error: unknown): AuthErrorResponse | null {
  if (typeof error === 'string') {
    try {
      return JSON.parse(error) as AuthErrorResponse;
    } catch {
      return null;
    }
  }
  return null;
}

/**
 * Check if an error is a specific auth error code
 */
export function isAuthErrorCode(error: unknown, code: AuthErrorResponse['code']): boolean {
  const parsed = parseAuthError(error);
  return parsed?.code === code;
}

// ============================================================================
// Core Auth Commands
// ============================================================================

export async function invokeSignIn(
  email: string,
  password: string,
): Promise<AuthResponse> {
  return invoke<AuthResponse>('sign_in_with_password', {
    email,
    password,
  });
}

export async function invokeSignUp(request: SignUpRequest): Promise<SignUpResponse> {
  return invoke<SignUpResponse>('sign_up', {
    email: request.email,
    password: request.password,
    fullName: request.full_name,
  });
}

// ============================================================================
// Google OAuth Commands
// ============================================================================

/**
 * Initiates Google OAuth sign-in by getting the authorization URL
 * The URL should be opened in the system browser
 */
export async function invokeSignInWithGoogle(fullName?: string): Promise<OAuthUrlResponse> {
  return invoke<OAuthUrlResponse>('sign_in_with_google', {
    fullName,
  });
}

export async function invokeVerifyOtp(
  email: string,
  token: string,
  otpType?: string,
): Promise<VerifyOtpResponse> {
  return invoke<VerifyOtpResponse>('verify_otp', {
    email,
    token,
    otpType: otpType ?? 'signup',
  });
}

export async function invokeResendConfirmation(
  email: string,
): Promise<ResendConfirmationResponse> {
  return invoke<ResendConfirmationResponse>('resend_confirmation', {
    email,
  });
}

export async function invokeRefreshToken(): Promise<RefreshTokenResponse> {
  return invoke<RefreshTokenResponse>('refresh_token');
}

export async function invokeGetAuthState(): Promise<AuthState> {
  return invoke<AuthState>('get_auth_state');
}

/**
 * Get full auth state in a single call
 * Returns: isOnline, isAuthenticated, isSetupComplete, user, needsRefresh, expiresAt
 */
export async function invokeGetFullAuthState(): Promise<FullAuthState> {
  return invoke<FullAuthState>('get_full_auth_state');
}

export async function invokeGetAccessToken(): Promise<string | null> {
  return invoke<string | null>('get_access_token_command');
}

export async function invokeLogout(): Promise<string> {
  return invoke<string>('logout');
}

export async function invokeIsAuthenticated(): Promise<boolean> {
  return invoke<boolean>('is_authenticated');
}

export async function invokeIsSetupComplete(): Promise<boolean> {
  return invoke<boolean>('check_setup_complete');
}

export async function invokeEmitAuthChanged(): Promise<void> {
  return invoke<void>('emit_auth_changed');
}

export async function invokeGetCurrentUser(): Promise<UserInfo | null> {
  return invoke<UserInfo | null>('get_current_user');
}

export async function invokeIsOnline(): Promise<boolean> {
  return invoke<boolean>('is_online');
}