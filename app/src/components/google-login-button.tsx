"use client";

import { Button } from "@/components/ui/button";
import { getAuthErrorMessage, useRoleAccess } from "@/lib/role-access";
import { invokeEmitAuthChanged } from "@/lib/role-access/commands";
import { AlertCircle, Loader2 } from "lucide-react";
import { useEffect, useState } from "react";

const googleLogo = "/google-logo.png";

const GoogleIcon = () => (
  <img src={googleLogo} alt="Google Logo" className="w-5 h-5" />
);

interface GoogleLoginButtonProps {
  onSignInSuccess: () => void;
  variant?: "default" | "outline" | "secondary" | "ghost";
  size?: "default" | "sm" | "lg" | "icon";
  className?: string;
  disabled?: boolean;
}

export function GoogleLoginButton({
  onSignInSuccess,
  variant = "outline",
  size = "default",
  className = "",
  disabled = false,
}: GoogleLoginButtonProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const { signInWithGoogle } = useRoleAccess();

  useEffect(() => {
    const listenToOAuth2Events = async () => {
      const { listen } = await import("@tauri-apps/api/event");

      const unlistenSuccess = await listen("oauth2-success", () => {
        void (async () => {
          setIsLoading(false);
          setError(null);
          await invokeEmitAuthChanged();
          onSignInSuccess();
        })();
      });

      const unlistenError = await listen("oauth2-error", (event) => {
        console.error("[GoogleLoginButton] OAuth2 error:", event.payload);
        setIsLoading(false);
        setError(
          getAuthErrorMessage(
            event.payload,
            "Sign-in with Google failed. Please try again."
          )
        );
      });

      return () => {
        unlistenSuccess();
        unlistenError();
      };
    };

    let cleanup: (() => void) | undefined;
    void listenToOAuth2Events().then((cleanupFn) => {
      cleanup = cleanupFn;
    });

    return () => {
      if (cleanup) cleanup();
    };
  }, [onSignInSuccess]);

  const handleGoogleSignIn = async () => {
    setError(null);
    setIsLoading(true);

    try {
      await signInWithGoogle();
    } catch (err) {
      console.error("Failed to initiate Google sign in:", err);
      setError(
        getAuthErrorMessage(
          err,
          "Failed to start Google sign in. Please try again."
        )
      );
      setIsLoading(false);
    }
  };

  return (
    <div className="w-full">
      <Button
        type="button"
        variant={variant}
        size={size}
        onClick={() => void handleGoogleSignIn()}
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
