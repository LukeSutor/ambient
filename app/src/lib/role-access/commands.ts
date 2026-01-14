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
} from './types';

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
    givenName: request.given_name,
    familyName: request.family_name,
  });
}

// ============================================================================
// Google OAuth Commands
// ============================================================================

/**
 * Initiates Google OAuth sign-in by getting the authorization URL
 * The URL should be opened in the system browser
 */
export async function invokeSignInWithGoogle(givenName?: string, familyName?: string): Promise<OAuthUrlResponse> {
  return invoke<OAuthUrlResponse>('sign_in_with_google', {
    givenName,
    familyName,
  });
}

/**
 * Exchange an OAuth authorization code for a session
 * This is called internally when the deep link callback is received
 */
export async function invokeExchangeCodeForSession(code: string): Promise<AuthResponse> {
  return invoke<AuthResponse>('exchange_code_for_session', {
    code,
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