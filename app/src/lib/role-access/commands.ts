import { invoke } from '@tauri-apps/api/core';
import type {
  SignUpRequest,
  AuthResponse,
  SignUpResponse,
  ResendConfirmationResponse,
  VerifyOtpResponse,
  OAuthUrlResponse,
  AuthState,
  AuthErrorResponse,
} from './types';

// ============================================================================
// Error Handling Helpers
// ============================================================================

/**
 * User-friendly error messages for each error code
 */
const ERROR_MESSAGES: Record<AuthErrorResponse['code'], string> = {
  network_error: 'Unable to connect. Please check your internet connection.',
  invalid_credentials: 'Invalid email or password. Please try again.',
  email_not_confirmed: 'Please verify your email address before signing in.',
  user_already_exists: 'An account with this email already exists.',
  invalid_otp: 'The verification code is invalid or has expired.',
  rate_limited: 'Too many attempts.',
  o_auth_error: 'Sign-in with Google failed. Please try again.',
  session_expired: 'Your session has expired. Please sign in again.',
  invalid_request: 'Invalid request. Please check your input and try again.',
  server_error: 'A server error occurred. Please try again later.',
  storage_error: 'Failed to save your session. Please try again.',
  unknown: 'An unexpected error occurred. Please try again.',
};

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
 * Get a user-friendly error message from an auth error
 * Falls back to the error's message, details, or a default message
 */
export function getAuthErrorMessage(error: unknown, defaultMessage: string): string {
  const parsed = parseAuthError(error);
  
  if (parsed) {
    // Use the user-friendly message for the error code if available
    const friendlyMessage = ERROR_MESSAGES[parsed.code];
    
    // For some codes, prefer the backend message if it's more specific
    if (parsed.code === 'invalid_credentials' || 
        parsed.code === 'email_not_confirmed' ||
        parsed.code === 'user_already_exists') {
      return parsed.message || friendlyMessage;
    }
    
    // For rate limiting, include details if available
    if (parsed.code === 'rate_limited' && parsed.details) {
      return `${friendlyMessage} ${parsed.details}`;
    }
    
    return friendlyMessage || parsed.message || defaultMessage;
  }
  
  return defaultMessage;
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
export async function invokeSignInWithGoogle(): Promise<OAuthUrlResponse> {
  return invoke<OAuthUrlResponse>('sign_in_with_google');
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

/**
 * Get full auth state in a single call
 * Returns: isOnline, isAuthenticated, isSetupComplete, user, needsRefresh, expiresAt
 */
export async function invokeGetAuthState(): Promise<AuthState> {
  return invoke<AuthState>('get_auth_state');
}

export async function invokeLogout(): Promise<string> {
  return invoke<string>('logout');
}

export async function invokeIsSetupComplete(): Promise<boolean> {
  return invoke<boolean>('check_setup_complete');
}

export async function invokeEmitAuthChanged(): Promise<void> {
  return invoke<void>('emit_auth_changed');
}