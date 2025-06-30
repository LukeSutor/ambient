"use client";
import React, { useState, useEffect } from 'react';
import { AuthService } from '@/lib/auth';
import { Button } from '@/components/ui/button';
import { Loader2, AlertCircle } from 'lucide-react';
import { useRouter } from 'next/navigation';

// Google logo SVG component
const GoogleIcon = () => (
  <svg 
    className="w-5 h-5" 
    viewBox="0 0 24 24" 
    fill="none" 
    xmlns="http://www.w3.org/2000/svg"
  >
    <path 
      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" 
      fill="#4285F4"
    />
    <path 
      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" 
      fill="#34A853"
    />
    <path 
      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z" 
      fill="#FBBC05"
    />
    <path 
      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z" 
      fill="#EA4335"
    />
  </svg>
);

interface GoogleLoginButtonProps {
  onSignInSuccess?: () => void;
  variant?: 'default' | 'outline' | 'secondary' | 'ghost';
  size?: 'default' | 'sm' | 'lg' | 'icon';
  className?: string;
  disabled?: boolean;
}

export function GoogleLoginButton({ 
  onSignInSuccess, 
  variant = 'outline',
  size = 'default',
  className = '',
  disabled = false 
}: GoogleLoginButtonProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const router = useRouter();

  useEffect(() => {
    // Listen for OAuth2 events from Tauri
    const listenToOAuth2Events = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      
      // Listen for OAuth2 success
      const unlistenSuccess = await listen('oauth2-success', (event) => {
        console.log('OAuth2 success:', event.payload);
        setIsLoading(false);
        setError(null);
        
        if (onSignInSuccess) {
          onSignInSuccess();
        } else {
          router.push('/');
        }
      });

      // Listen for OAuth2 errors
      const unlistenError = await listen('oauth2-error', (event) => {
        console.error('OAuth2 error:', event.payload);
        setIsLoading(false);
        setError(event.payload as string || 'Authentication failed');
      });

      // Return cleanup function
      return () => {
        unlistenSuccess();
        unlistenError();
      };
    };

    let cleanup: (() => void) | undefined;
    listenToOAuth2Events().then((cleanupFn) => {
      cleanup = cleanupFn;
    });

    return () => {
      if (cleanup) cleanup();
    };
  }, [onSignInSuccess, router]);

  const handleGoogleSignIn = async () => {
    setError(null);
    setIsLoading(true);

    try {
      // Get the authorization URL from the backend
      const authUrl = await AuthService.initiateGoogleAuth();
      console.log('Opening Google OAuth URL:', authUrl);
      
      // Open the URL in the default browser
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      await openUrl(authUrl);
      
      // Note: The actual authentication will be handled by the deep link callback
      // The loading state will be cleared by the event listeners above
    } catch (err) {
      console.error('Failed to initiate Google sign in:', err);
      setError(err as string || 'Failed to start Google sign in');
      setIsLoading(false);
    }
  };

  return (
    <div className="w-full">
      <Button
        type="button"
        variant={variant}
        size={size}
        onClick={handleGoogleSignIn}
        disabled={disabled || isLoading}
        className={`w-full ${className}`}
      >
        {isLoading ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            Connecting to Google...
          </>
        ) : (
          <>
            <GoogleIcon />
            <span className="ml-2">Continue with Google</span>
          </>
        )}
      </Button>
      
      {error && (
        <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md mt-2">
          <AlertCircle className="h-4 w-4" />
          <span className="text-sm">{error}</span>
        </div>
      )}
    </div>
  );
}
