import { invoke } from '@tauri-apps/api/core';
import type {
  UserInfo,
  SignInResult,
  ConfirmSignUpRequest,
  SignUpRequest,
  SignUpResult,
} from './types';

export async function invokeGoogleSignOut(): Promise<void> {
  return invoke<void>('google_sign_out');
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

export async function invokeCognitoSignIn(
  username: string,
  password: string,
): Promise<SignInResult> {
  return invoke<SignInResult>('cognito_sign_in', {
    username,
    password,
  });
}

export async function invokeGoogleSignIn(): Promise<SignInResult> {
  return invoke<SignInResult>('google_sign_in');
}

export async function invokeCognitoSignUp(request: SignUpRequest): Promise<SignUpResult> {
  return invoke<SignUpResult>('cognito_sign_up', {
    email: request.email,
    password: request.password,
    givenName: request.given_name,
    familyName: request.family_name,
  });
}

export async function invokeCognitoConfirmSignUp(
  request: ConfirmSignUpRequest,
): Promise<void> {
  console.log('Invoking cognito_confirm_sign_up with', request);
  return invoke('cognito_confirm_sign_up', {
    email: request.email,
    confirmationCode: request.confirmation_code,
  });
}

export async function invokeCognitoResendConfirmationCode(
  username: string,
): Promise<SignUpResult> {
  return invoke<SignUpResult>('cognito_resend_confirmation_code', {
    username,
  });
}
