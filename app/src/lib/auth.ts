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

// Auth service for interacting with Tauri backend
export class AuthService {
  /**
   * Logs out the user and clears stored tokens
   */
  static async logout(): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('logout');
  }

  /**
   * Comprehensive logout that handles both regular and Google authentication
   * This function attempts to log out from all possible authentication methods
   */
  static async logoutAll(): Promise<string> {
    try {
      // First, check if user is authenticated and get user info to determine auth method
      const isAuth = await this.isAuthenticated();
      if (!isAuth) {
        console.log('AuthService.logoutAll: User already logged out');
        return 'User already logged out';
      }

      console.log('AuthService.logoutAll: Starting comprehensive logout process');

      // Try Google logout first (this is safe to call even if not Google authenticated)
      try {
        console.log('AuthService.logoutAll: Attempting Google sign out');
        await this.googleSignOut();
        console.log('AuthService.logoutAll: Google sign out completed');
      } catch (error) {
        console.warn('AuthService.logoutAll: Google sign out failed or not applicable:', error);
      }

      // Then perform regular logout to clear all tokens
      console.log('AuthService.logoutAll: Attempting regular logout');
      const result = await this.logout();
      console.log('AuthService.logoutAll: Regular logout completed:', result);
      return result;
    } catch (error) {
      console.error('AuthService.logoutAll: Logout failed:', error);
      // Still try regular logout as fallback
      try {
        console.log('AuthService.logoutAll: Attempting fallback logout');
        return await this.logout();
      } catch (fallbackError) {
        console.error('AuthService.logoutAll: Fallback logout also failed:', fallbackError);
        throw fallbackError;
      }
    }
  }

  /**
   * Retrieves the stored authentication token
   */
  static async getStoredToken(): Promise<AuthToken | null> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<AuthToken | null>('get_stored_token');
  }

  /**
   * Checks if the user is currently authenticated
   */
  static async isAuthenticated(): Promise<boolean> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<boolean>('is_authenticated');
  }

  /**
   * Sign in with username and password using Cognito
   */
  static async signIn(username: string, password: string): Promise<SignInResult> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SignInResult>('cognito_sign_in', {
      username,
      password,
    });
  }

  /**
   * Get current user information
   */
  static async getCurrentUser(): Promise<CognitoUserInfo | null> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<CognitoUserInfo | null>('get_current_user');
  }

  /**
   * Sign up a new user with AWS Cognito
   */
  static async signUp(request: SignUpRequest): Promise<SignUpResult> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SignUpResult>('cognito_sign_up', {
      username: request.username,
      password: request.password,
      email: request.email,
      givenName: request.given_name,
      familyName: request.family_name,
    });
  }

  /**
   * Confirm user sign up with verification code
   */
  static async confirmSignUp(
    username: string,
    confirmationCode: string,
    session?: string
  ): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('cognito_confirm_sign_up', {
      username,
      confirmationCode,
      session,
    });
  }

  /**
   * Resend confirmation code for user verification
   */
  static async resendConfirmationCode(username: string): Promise<SignUpResult> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<SignUpResult>('cognito_resend_confirmation_code', {
      username,
    });
  }

  /**
   * Gets the access token for API requests
   * Returns null if not authenticated or token is expired
   */
  static async getAccessToken(): Promise<string | null> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string | null>('get_access_token');
  }

  /**
   * Creates an Authorization header for API requests
   */
  static async getAuthorizationHeader(): Promise<{ Authorization: string } | null> {
    const token = await this.getAccessToken();
    if (!token) return null;
    
    return {
      Authorization: `Bearer ${token}`
    };
  }

  /**
   * Initiate Google OAuth2 authentication
   * Returns the authorization URL to open in browser/external app
   */
  static async initiateGoogleAuth(): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('google_initiate_auth');
  }

  /**
   * Sign in with Google (simplified - handles URL generation and opening in backend)
   */
  static async signInWithGoogle(): Promise<void> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<void>('google_sign_in');
  }

  /**
   * Sign out from Google OAuth2
   */
  static async googleSignOut(): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('google_sign_out');
  }

  /**
   * Determines the authentication method used by checking user info patterns
   * This is a best-effort detection based on available user data
   */
  static async getAuthenticationMethod(): Promise<'google' | 'cognito' | 'unknown'> {
    try {
      const user = await this.getCurrentUser();
      if (!user) return 'unknown';

      // Google OAuth users typically have email as username or specific patterns
      // This is a heuristic and may need adjustment based on actual data patterns
      if (user.username && user.username.startsWith('google-')) {
        return 'google';
      }

      // If we have detailed user info with given_name/family_name but no email-like username,
      // it's likely a regular Cognito user
      if (user.given_name || user.family_name) {
        return 'cognito';
      }

      // Default to cognito for regular usernames
      return 'cognito';
    } catch (error) {
      console.error('Failed to determine authentication method:', error);
      return 'unknown';
    }
  }
}
