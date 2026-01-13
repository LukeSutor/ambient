import { invoke } from '@tauri-apps/api/core';
import type {
  UserInfo,
  SignInResult,
  ConfirmSignUpRequest,
  SignUpRequest,
  AuthResponse,
  SignUpResponse,
  AuthState,
  RefreshTokenResponse,
  ResendConfirmationResponse,
  VerifyOtpResponse,
} from './types';

// ============================================================================
// Core Auth Commands (New)
// ============================================================================

export async function invokeSignIn(
  email: string,
  password: string,
): Promise<AuthResponse> {
  return invoke<AuthResponse>('sign_in', {
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

// ============================================================================
// Legacy Commands (for backward compatibility)
// ============================================================================

/**
 * @deprecated Use invokeSignIn instead
 */
export async function invokeCognitoSignIn(
  email: string,
  password: string,
): Promise<SignInResult> {
  // Call the new sign_in command and transform the response to match legacy format
  const response = await invokeSignIn(email, password);
  
  // Transform AuthResponse to legacy SignInResult
  return {
    access_token: response.session?.access_token ?? '',
    id_token: undefined, // Supabase doesn't use separate id_token
    refresh_token: response.session?.refresh_token,
    expires_in: Number(response.session?.expires_in ?? 0),
    user_info: response.user ? {
      id: response.user.id,
      email: response.user.email,
      given_name: response.user.user_metadata?.given_name ?? null,
      family_name: response.user.user_metadata?.family_name ?? null,
      email_verified: response.user.user_metadata?.email_verified ?? null,
      provider: response.user.app_metadata?.provider ?? null,
      created_at: response.user.created_at ?? null,
    } : {
      id: '',
      email: null,
      given_name: null,
      family_name: null,
      email_verified: null,
      provider: null,
      created_at: null,
    },
  };
}

export async function invokeGoogleSignIn(): Promise<SignInResult> {
  return invoke<SignInResult>('google_sign_in');
}

/**
 * @deprecated Use invokeSignUp instead
 */
export async function invokeCognitoSignUp(request: SignUpRequest): Promise<SignUpResponse> {
  return invokeSignUp(request);
}

/**
 * @deprecated Use invokeVerifyOtp instead
 */
export async function invokeCognitoConfirmSignUp(
  request: ConfirmSignUpRequest,
): Promise<void> {
  console.log('Invoking verify_otp with', request);
  await invokeVerifyOtp(request.email, request.confirmation_code, 'signup');
}

/**
 * @deprecated Use invokeResendConfirmation instead
 */
export async function invokeCognitoResendConfirmationCode(
  email: string,
): Promise<SignUpResponse> {
  const response = await invokeResendConfirmation(email);
  // Return a SignUpResponse-like object for compatibility
  return {
    user: null,
    session: null,
    verification_required: true,
    destination: email,
    delivery_medium: 'EMAIL',
  };
}
