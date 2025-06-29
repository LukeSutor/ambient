import { useState, useEffect } from 'react';

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
   * Initiates the OAuth2 authentication flow
   * Opens the browser for user authentication
   */
  static async authenticate(): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('authenticate');
  }

  /**
   * Logs out the user and clears stored tokens
   */
  static async logout(): Promise<string> {
    const { invoke } = await import('@tauri-apps/api/core');
    return invoke<string>('logout');
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
}

// React hook for authentication state
export function useAuth() {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean>(false);
  const [isLoading, setIsLoading] = useState<boolean>(true);
  const [token, setToken] = useState<AuthToken | null>(null);

  // Check authentication status on mount
  useEffect(() => {
    checkAuthStatus();
  }, []);

  const checkAuthStatus = async () => {
    try {
      setIsLoading(true);
      const authenticated = await AuthService.isAuthenticated();
      setIsAuthenticated(authenticated);
      
      if (authenticated) {
        const storedToken = await AuthService.getStoredToken();
        setToken(storedToken);
      }
    } catch (error) {
      console.error('Error checking auth status:', error);
      setIsAuthenticated(false);
      setToken(null);
    } finally {
      setIsLoading(false);
    }
  };

  const login = async () => {
    try {
      setIsLoading(true);
      await AuthService.authenticate();
      await checkAuthStatus(); // Refresh auth state
    } catch (error) {
      console.error('Login failed:', error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };

  const logout = async () => {
    try {
      setIsLoading(true);
      await AuthService.logout();
      setIsAuthenticated(false);
      setToken(null);
    } catch (error) {
      console.error('Logout failed:', error);
      throw error;
    } finally {
      setIsLoading(false);
    }
  };

  return {
    isAuthenticated,
    isLoading,
    token,
    login,
    logout,
    checkAuthStatus,
  };
}
