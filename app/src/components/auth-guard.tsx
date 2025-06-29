"use client";

import { useEffect, useState, ReactNode } from 'react';
import { AuthService } from '@/lib/auth';

interface AuthGuardProps {
  children: ReactNode;
  fallback?: ReactNode;
}

export function AuthGuard({ children, fallback }: AuthGuardProps) {
  const [isAuthenticated, setIsAuthenticated] = useState<boolean | null>(null);

  useEffect(() => {
    const checkAuth = async () => {
      try {
        const authenticated = await AuthService.isAuthenticated();
        if (!authenticated) {
          window.location.href = '/signin';
          return;
        }
        setIsAuthenticated(true);
      } catch (error) {
        console.error('Auth check failed:', error);
        window.location.href = '/signin';
      }
    };

    checkAuth();
  }, []);

  if (isAuthenticated === null) {
    return (
      fallback || (
        <div className="flex h-screen w-full items-center justify-center">
          <div className="text-center">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-gray-900 mx-auto mb-4"></div>
            <p>Checking authentication...</p>
          </div>
        </div>
      )
    );
  }

  return <>{children}</>;
}
