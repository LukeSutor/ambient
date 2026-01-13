// Re-export types from generated auth types
import type {
  AuthResponse,
  AuthState,
  SignUpResponse,
  UserInfo as GeneratedUserInfo,
  VerifyOtpResponse,
  RefreshTokenResponse,
  ResendConfirmationResponse,
  Session,
  SupabaseUser,
} from '@/types/auth';

// Re-export all auth types
export type {
  AuthResponse,
  AuthState,
  SignUpResponse,
  VerifyOtpResponse,
  RefreshTokenResponse,
  ResendConfirmationResponse,
  Session,
  SupabaseUser,
} from '@/types/auth';

/**
 * Role access state
 */
export interface RoleAccessState {
  isHydrated: boolean;
  isOnline: boolean;
  isLoggedIn: boolean;
  isSetupComplete: boolean;
  isPremiumUser: boolean;
  userInfo: UserInfo | null;
}

/**
 * User info exposed to the frontend - matches the generated UserInfo type
 */
export interface UserInfo {
  id: string;
  email: string | null;
  given_name: string | null;
  family_name: string | null;
  email_verified: boolean | null;
  provider: string | null;
  created_at: string | null;
}

// Alias for backward compatibility
export type CognitoUserInfo = UserInfo;

/**
 * Alias for backward compatibility - use SignUpResponse instead
 */
export type SignUpResult = SignUpResponse;

// ============================================================================
// Request Types (for frontend use)
// ============================================================================

export interface SignUpRequest {
  email: string;
  password: string;
  given_name?: string;
  family_name?: string;
}

export interface ConfirmSignUpRequest {
  email: string;
  confirmation_code: string;
}

export interface SignInRequest {
  email: string;
  password: string;
}