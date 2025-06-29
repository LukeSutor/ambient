import { AuthService } from './auth';

/**
 * Utility class for making authenticated requests to AWS API Gateway
 * Automatically includes the Cognito JWT token in requests
 */
export class AwsApiClient {
  private baseUrl: string;

  constructor(baseUrl: string) {
    this.baseUrl = baseUrl.replace(/\/$/, ''); // Remove trailing slash
  }

  /**
   * Make an authenticated GET request
   */
  async get<T>(endpoint: string): Promise<T> {
    return this.request<T>('GET', endpoint);
  }

  /**
   * Make an authenticated POST request
   */
  async post<T>(endpoint: string, data?: any): Promise<T> {
    return this.request<T>('POST', endpoint, data);
  }

  /**
   * Make an authenticated PUT request
   */
  async put<T>(endpoint: string, data?: any): Promise<T> {
    return this.request<T>('PUT', endpoint, data);
  }

  /**
   * Make an authenticated DELETE request
   */
  async delete<T>(endpoint: string): Promise<T> {
    return this.request<T>('DELETE', endpoint);
  }

  /**
   * Generic method for making authenticated requests
   */
  private async request<T>(
    method: string,
    endpoint: string,
    data?: any
  ): Promise<T> {
    // Ensure user is authenticated
    const isAuthenticated = await AuthService.isAuthenticated();
    if (!isAuthenticated) {
      throw new Error('User is not authenticated');
    }

    // Get authorization header
    const authHeader = await AuthService.getAuthorizationHeader();
    if (!authHeader) {
      throw new Error('Failed to get authorization token');
    }

    // Prepare request
    const url = `${this.baseUrl}${endpoint}`;
    const headers: HeadersInit = {
      'Content-Type': 'application/json',
      ...authHeader,
    };

    const config: RequestInit = {
      method,
      headers,
    };

    if (data && (method === 'POST' || method === 'PUT')) {
      config.body = JSON.stringify(data);
    }

    // Make request
    const response = await fetch(url, config);

    // Handle response
    if (!response.ok) {
      if (response.status === 401) {
        throw new Error('Unauthorized - token may be expired');
      }
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    // Parse JSON response
    const contentType = response.headers.get('content-type');
    if (contentType && contentType.includes('application/json')) {
      return response.json();
    }

    // Return as text if not JSON
    return response.text() as T;
  }
}

/**
 * Example usage for your specific API
 * Replace with your actual API Gateway URL
 */
export const apiClient = new AwsApiClient(
  process.env.NEXT_PUBLIC_API_URL || 'https://your-api-gateway-url.amazonaws.com/prod'
);

/**
 * Typed interfaces for your API responses
 * Add your own types here based on your API
 */
export interface UserProfile {
  id: string;
  email: string;
  name?: string;
  created_at: string;
  updated_at: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data: T;
  message?: string;
}

/**
 * Example API service methods
 * Replace these with your actual API endpoints
 */
export class ApiService {
  /**
   * Get current user profile
   */
  static async getUserProfile(): Promise<UserProfile> {
    const response = await apiClient.get<ApiResponse<UserProfile>>('/user/profile');
    return response.data;
  }

  /**
   * Update user profile
   */
  static async updateUserProfile(profile: Partial<UserProfile>): Promise<UserProfile> {
    const response = await apiClient.put<ApiResponse<UserProfile>>('/user/profile', profile);
    return response.data;
  }

  /**
   * Example: Get user's data
   */
  static async getUserData(): Promise<any[]> {
    const response = await apiClient.get<ApiResponse<any[]>>('/user/data');
    return response.data;
  }

  /**
   * Example: Create new data item
   */
  static async createDataItem(item: any): Promise<any> {
    const response = await apiClient.post<ApiResponse<any>>('/user/data', item);
    return response.data;
  }
}

/**
 * Helper to handle API errors in React components
 */
export function handleApiError(error: unknown): string {
  if (error instanceof Error) {
    if (error.message.includes('not authenticated')) {
      return 'Please log in to continue';
    }
    if (error.message.includes('Unauthorized')) {
      return 'Your session has expired. Please log in again';
    }
    return error.message;
  }
  return 'An unexpected error occurred';
}
