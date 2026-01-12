/**
 * Role access state
 */
export interface RoleAccessState {
  isHydrated: boolean;
  isOnline: boolean;
  isLoggedIn: boolean;
  isSetupComplete: boolean;
  isPremiumUser: boolean;
  userInfo: CognitoUserInfo | null;
}

// Auth types for Tauri commands
export interface AuthToken {
  access_token: string;
  refresh_token?: string;
  id_token?: string;
  expires_in?: number; // Duration in seconds
}

export interface SignUpResult {
  user_sub: string;
  user_confirmed: boolean;
  verification_required: boolean;
  destination?: string;
  delivery_medium?: string;
  session?: string;
}

export interface SignUpRequest {
  username: string;
  password: string;
  email: string;
  given_name?: string;
  family_name?: string;
}

export interface ConfirmSignUpRequest {
  username: string;
  confirmation_code: string;
  session?: string;
}

export interface CognitoUserInfo {
  username: string;
  email?: string;
  given_name?: string;
  family_name?: string;
  sub: string;
}

export interface SignInResult {
  access_token: string;
  id_token: string;
  refresh_token: string;
  expires_in: number;
  user_info: CognitoUserInfo;
}