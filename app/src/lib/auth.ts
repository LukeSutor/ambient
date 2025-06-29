import { useState, useEffect } from 'react';

// Auth types for Tauri commands
export interface AuthToken {
  access_token: string;
  refresh_token?: string;
  id_token?: string;
  expires_in?: number; // Duration in seconds
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
   * Gets the access token for API requests
   * Returns null if not authenticated
   */
  static async getAccessToken(): Promise<string | null> {
    try {
      const token = await this.getStoredToken();
      return token?.access_token || null;
    } catch {
      return null;
    }
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
