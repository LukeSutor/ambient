// Re-export types from generated auth types
import type {
  UserInfo,
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
  UserInfo,
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

// ============================================================================
// Request Types for frontend use
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