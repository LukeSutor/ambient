// Export hooks and types
export * from './useRoleAccess';
export * from './types';

// Export error handling utilities
export { 
  parseAuthError, 
  isAuthErrorCode, 
  getAuthErrorMessage 
} from './commands';