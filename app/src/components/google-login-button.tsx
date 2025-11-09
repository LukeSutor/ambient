"use client";
import React, { useState, useEffect } from 'react';
import { useRoleAccess } from '@/lib/role-access';
import { Button } from '@/components/ui/button';
import { Loader2, AlertCircle } from 'lucide-react';
import { useRouter } from 'next/navigation';
const googleLogo = "/google-logo.png";

// Google logo SVG component
const GoogleIcon = () => (
  <img src={googleLogo} alt="Google Logo" className="w-5 h-5" />
);

interface GoogleLoginButtonProps {
  onSignInSuccess: () => void;
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

  // Auth state
  const { googleSignIn } = useRoleAccess();

  useEffect(() => {
    // Listen for OAuth2 events from Tauri
    const listenToOAuth2Events = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      
      // Listen for OAuth2 success
      const unlistenSuccess = await listen('oauth2-success', (event) => {
        setIsLoading(false);
        setError(null);
        onSignInSuccess();
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
      await googleSignIn();
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
