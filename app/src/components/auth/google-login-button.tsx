"use client";

import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { getAuthErrorMessage, useRoleAccess } from "@/lib/role-access";
import { invokeEmitAuthChanged } from "@/lib/role-access/commands";
import { AlertCircle, Loader2 } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

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

  // Use refs to avoid re-registering listeners when callbacks change
  const onSignInSuccessRef = useRef(onSignInSuccess);
  onSignInSuccessRef.current = onSignInSuccess;

  useEffect(() => {
    let cancelled = false;
    let unlistenSuccess: (() => void) | undefined;
    let unlistenError: (() => void) | undefined;

    const setup = async () => {
      const { listen } = await import("@tauri-apps/api/event");

      if (cancelled) return;

      unlistenSuccess = await listen("oauth2-success", () => {
        if (cancelled) return;
        void (async () => {
          setIsLoading(false);
          setError(null);
          // Open the user's encrypted database after OAuth login
          await invoke("open_user_database");
          await invokeEmitAuthChanged();
          onSignInSuccessRef.current();
        })();
      });

      if (cancelled) {
        unlistenSuccess();
        return;
      }

      unlistenError = await listen("oauth2-error", (event) => {
        if (cancelled) return;
        console.error("[GoogleLoginButton] OAuth2 error:", event.payload);
        setIsLoading(false);
        setError(
          getAuthErrorMessage(
            event.payload,
            "Sign-in with Google failed. Please try again.",
          ),
        );
      });

      if (cancelled) {
        unlistenSuccess?.();
        unlistenError();
        return;
      }
    };

    void setup();

    return () => {
      cancelled = true;
      unlistenSuccess?.();
      unlistenError?.();
    };
    // Empty dependency array — listeners are stable, refs handle callback updates
  }, []);

  const handleGoogleSignIn = useCallback(async () => {
    setError(null);
    setIsLoading(true);

    try {
      await signInWithGoogle();
    } catch (err) {
      console.error("Failed to initiate Google sign in:", err);
      setError(
        getAuthErrorMessage(
          err,
          "Failed to start Google sign in. Please try again.",
        ),
      );
      setIsLoading(false);
    }
  }, [signInWithGoogle]);

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
