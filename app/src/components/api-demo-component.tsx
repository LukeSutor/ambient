"use client";
import React, { useState, useEffect } from 'react';
import { useAuth } from '@/lib/auth';
import { ApiService, handleApiError, UserProfile } from '@/lib/api-client';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Loader2, User, RefreshCw, AlertCircle } from 'lucide-react';

export function ApiDemoComponent() {
  const { isAuthenticated, isLoading: authLoading } = useAuth();
  const [userProfile, setUserProfile] = useState<UserProfile | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Load user profile when authenticated
  useEffect(() => {
    if (isAuthenticated) {
      loadUserProfile();
    } else {
      setUserProfile(null);
      setError(null);
    }
  }, [isAuthenticated]);

  const loadUserProfile = async () => {
    try {
      setIsLoading(true);
      setError(null);
      const profile = await ApiService.getUserProfile();
      setUserProfile(profile);
    } catch (err) {
      setError(handleApiError(err));
    } finally {
      setIsLoading(false);
    }
  };

  const refreshProfile = async () => {
    await loadUserProfile();
  };

  if (authLoading) {
    return (
      <Card className="w-full max-w-md mx-auto">
        <CardContent className="flex items-center justify-center p-6">
          <Loader2 className="h-6 w-6 animate-spin" />
          <span className="ml-2">Loading...</span>
        </CardContent>
      </Card>
    );
  }

  if (!isAuthenticated) {
    return (
      <Card className="w-full max-w-md mx-auto">
        <CardHeader>
          <CardTitle className="flex items-center">
            <AlertCircle className="h-5 w-5 mr-2 text-yellow-500" />
            Authentication Required
          </CardTitle>
          <CardDescription>
            Please authenticate first to access the API demo
          </CardDescription>
        </CardHeader>
      </Card>
    );
  }

  return (
    <div className="w-full max-w-2xl mx-auto space-y-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center">
            <User className="h-5 w-5 mr-2" />
            API Demo - User Profile
          </CardTitle>
          <CardDescription>
            This demo shows how to make authenticated API calls to AWS services
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {error && (
            <div className="p-3 bg-red-50 border border-red-200 rounded-md flex items-center">
              <AlertCircle className="h-4 w-4 text-red-500 mr-2" />
              <span className="text-red-700">{error}</span>
            </div>
          )}

          {isLoading ? (
            <div className="flex items-center justify-center p-6">
              <Loader2 className="h-6 w-6 animate-spin" />
              <span className="ml-2">Loading user profile...</span>
            </div>
          ) : userProfile ? (
            <div className="space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <label className="text-sm font-medium text-gray-500">User ID</label>
                  <p className="text-sm font-mono bg-gray-100 p-2 rounded">
                    {userProfile.id}
                  </p>
                </div>
                <div>
                  <label className="text-sm font-medium text-gray-500">Email</label>
                  <p className="text-sm">{userProfile.email}</p>
                </div>
                {userProfile.name && (
                  <div>
                    <label className="text-sm font-medium text-gray-500">Name</label>
                    <p className="text-sm">{userProfile.name}</p>
                  </div>
                )}
                <div>
                  <label className="text-sm font-medium text-gray-500">Created At</label>
                  <p className="text-sm">
                    {new Date(userProfile.created_at).toLocaleDateString()}
                  </p>
                </div>
              </div>
            </div>
          ) : (
            <div className="text-center p-6 text-gray-500">
              <p>No profile data available</p>
              <p className="text-sm mt-2">
                This might happen if your API endpoint is not configured or accessible
              </p>
            </div>
          )}

          <div className="flex justify-between items-center pt-4 border-t">
            <Button
              onClick={refreshProfile}
              variant="outline"
              disabled={isLoading}
              className="flex items-center"
            >
              <RefreshCw className={`h-4 w-4 mr-2 ${isLoading ? 'animate-spin' : ''}`} />
              Refresh Profile
            </Button>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Integration Instructions</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-3 text-sm">
            <div>
              <h4 className="font-medium">1. Set up AWS API Gateway</h4>
              <p className="text-gray-600">
                Create API Gateway endpoints that use Cognito as an authorizer
              </p>
            </div>
            <div>
              <h4 className="font-medium">2. Configure Environment Variables</h4>
              <p className="text-gray-600">
                Set <code className="bg-gray-100 px-1 rounded">NEXT_PUBLIC_API_URL</code> to your API Gateway URL
              </p>
            </div>
            <div>
              <h4 className="font-medium">3. Update API Service</h4>
              <p className="text-gray-600">
                Modify the endpoints in <code className="bg-gray-100 px-1 rounded">src/lib/api-client.ts</code> to match your API
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
