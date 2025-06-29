import React from 'react';
import { useAuth } from '@/lib/auth';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader2, LogIn, LogOut, User } from 'lucide-react';

export function AuthComponent() {
  const { isAuthenticated, isLoading, token, login, logout } = useAuth();

  if (isLoading) {
    return (
      <Card className="w-full max-w-md mx-auto">
        <CardContent className="flex items-center justify-center p-6">
          <Loader2 className="h-6 w-6 animate-spin" />
          <span className="ml-2">Checking authentication...</span>
        </CardContent>
      </Card>
    );
  }

  if (isAuthenticated) {
    return (
      <Card className="w-full max-w-md mx-auto">
        <CardHeader>
          <CardTitle className="flex items-center">
            <User className="h-5 w-5 mr-2" />
            Authenticated
          </CardTitle>
          <CardDescription>
            You are successfully logged in with AWS Cognito
          </CardDescription>
        </CardHeader>
        <CardContent>
          {token && (
            <div className="space-y-2 text-sm">
              <div>
                <span className="font-medium">Access Token:</span>
                <div className="mt-1 p-2 bg-gray-100 rounded text-xs break-all">
                  {token.access_token.substring(0, 50)}...
                </div>
              </div>
              {token.refresh_token && (
                <div>
                  <span className="font-medium">Has Refresh Token:</span>
                  <span className="ml-2 text-green-600">Yes</span>
                </div>
              )}
              {token.expires_in && (
                <div>
                  <span className="font-medium">Expires In:</span>
                  <span className="ml-2">{token.expires_in} seconds</span>
                </div>
              )}
            </div>
          )}
        </CardContent>
        <CardFooter>
          <Button onClick={logout} variant="outline" className="w-full">
            <LogOut className="h-4 w-4 mr-2" />
            Logout
          </Button>
        </CardFooter>
      </Card>
    );
  }

  return (
    <Card className="w-full max-w-md mx-auto">
      <CardHeader>
        <CardTitle className="flex items-center">
          <LogIn className="h-5 w-5 mr-2" />
          Sign In Required
        </CardTitle>
        <CardDescription>
          Please authenticate with AWS Cognito to continue
        </CardDescription>
      </CardHeader>
      <CardFooter>
        <Button onClick={login} className="w-full">
          <LogIn className="h-4 w-4 mr-2" />
          Sign In with AWS Cognito
        </Button>
      </CardFooter>
    </Card>
  );
}
