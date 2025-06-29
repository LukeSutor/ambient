"use client";
import React, { useState } from 'react';
import { SignUpComponent } from '@/components/signup-component';
import { AuthComponent } from '@/components/auth-component';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { useAuth } from '@/lib/auth';
import { CheckCircle, LogIn, UserPlus } from 'lucide-react';

type AuthMode = 'signin' | 'signup';

export default function SignInPage() {
  const [mode, setMode] = useState<AuthMode>('signin');
  const [showSuccess, setShowSuccess] = useState(false);
  const { isAuthenticated } = useAuth();

  const handleSignUpSuccess = () => {
    setShowSuccess(true);
    // Optionally switch to sign-in mode after a delay
    setTimeout(() => {
      setMode('signin');
      setShowSuccess(false);
    }, 3000);
  };

  if (isAuthenticated) {
    return (
      <div className="container mx-auto p-6">
        <div className="max-w-md mx-auto">
          <Card>
            <CardHeader className="text-center">
              <CardTitle className="flex items-center justify-center text-green-600">
                <CheckCircle className="h-5 w-5 mr-2" />
                Already Authenticated
              </CardTitle>
              <CardDescription>
                You are already signed in to your account
              </CardDescription>
            </CardHeader>
            <CardContent>
              <AuthComponent />
            </CardContent>
          </Card>
        </div>
      </div>
    );
  }

  if (showSuccess) {
    return (
      <div className="container mx-auto p-6">
        <div className="max-w-md mx-auto">
          <Card>
            <CardHeader className="text-center">
              <CardTitle className="flex items-center justify-center text-green-600">
                <CheckCircle className="h-5 w-5 mr-2" />
                Account Created Successfully!
              </CardTitle>
              <CardDescription>
                Your account has been created and verified. You can now sign in.
              </CardDescription>
            </CardHeader>
            <CardContent className="text-center">
              <div className="animate-pulse text-sm text-gray-500">
                Redirecting to sign in...
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-6">
      <div className="max-w-4xl mx-auto">
        <div className="text-center mb-8">
          <h1 className="text-4xl font-bold text-gray-900 mb-4">
            Welcome to Local Computer Use
          </h1>
          <p className="text-lg text-gray-600">
            Sign in to your account or create a new one to get started
          </p>
        </div>

        <div className="flex justify-center mb-6">
          <div className="bg-gray-100 p-1 rounded-lg inline-flex">
            <Button
              variant={mode === 'signin' ? 'default' : 'ghost'}
              onClick={() => setMode('signin')}
              className="flex items-center"
            >
              <LogIn className="h-4 w-4 mr-2" />
              Sign In
            </Button>
            <Button
              variant={mode === 'signup' ? 'default' : 'ghost'}
              onClick={() => setMode('signup')}
              className="flex items-center"
            >
              <UserPlus className="h-4 w-4 mr-2" />
              Sign Up
            </Button>
          </div>
        </div>

        <div className="grid gap-8 md:grid-cols-2">
          <div>
            {mode === 'signin' ? (
              <div>
                <h2 className="text-2xl font-semibold mb-4 text-center">Sign In</h2>
                <AuthComponent />
                <div className="mt-4 text-center">
                  <Button
                    variant="link"
                    onClick={() => setMode('signup')}
                    className="text-sm"
                  >
                    Don't have an account? Sign up
                  </Button>
                </div>
              </div>
            ) : (
              <div>
                <h2 className="text-2xl font-semibold mb-4 text-center">Create Account</h2>
                <SignUpComponent
                  onSignUpSuccess={handleSignUpSuccess}
                  onSwitchToLogin={() => setMode('signin')}
                />
              </div>
            )}
          </div>

          <div className="space-y-6">
            <Card>
              <CardHeader>
                <CardTitle>Why Create an Account?</CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="flex items-start space-x-3">
                  <CheckCircle className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <h4 className="font-medium">Secure Authentication</h4>
                    <p className="text-sm text-gray-600">Your data is protected with AWS Cognito's enterprise-grade security</p>
                  </div>
                </div>
                <div className="flex items-start space-x-3">
                  <CheckCircle className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <h4 className="font-medium">Personalized Experience</h4>
                    <p className="text-sm text-gray-600">Access your personal workflows and settings across devices</p>
                  </div>
                </div>
                <div className="flex items-start space-x-3">
                  <CheckCircle className="h-5 w-5 text-green-500 mt-0.5" />
                  <div>
                    <h4 className="font-medium">Cloud Sync</h4>
                    <p className="text-sm text-gray-600">Sync your data and preferences across all your devices</p>
                  </div>
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>Security Features</CardTitle>
              </CardHeader>
              <CardContent className="space-y-2 text-sm text-gray-600">
                <div className="flex items-center space-x-2">
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  <span>Email verification required</span>
                </div>
                <div className="flex items-center space-x-2">
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  <span>Strong password requirements</span>
                </div>
                <div className="flex items-center space-x-2">
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  <span>OAuth2 with PKCE protection</span>
                </div>
                <div className="flex items-center space-x-2">
                  <CheckCircle className="h-4 w-4 text-green-500" />
                  <span>Secure token storage</span>
                </div>
              </CardContent>
            </Card>
          </div>
        </div>
      </div>
    </div>
  );
}
