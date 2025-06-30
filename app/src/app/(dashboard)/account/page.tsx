"use client"

import { useState, useEffect } from 'react'
import { AuthService, CognitoUserInfo } from '@/lib/auth'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar'
import { Separator } from '@/components/ui/separator'
import { Badge } from '@/components/ui/badge'
import { Skeleton } from '@/components/ui/skeleton'
import { 
  User, 
  Mail, 
  Shield, 
  Clock, 
  Key,
  AlertCircle,
  CheckCircle2
} from 'lucide-react'
import { useRouter } from 'next/navigation'

export default function AccountPage() {
  const [user, setUser] = useState<CognitoUserInfo | null>(null)
  const [authMethod, setAuthMethod] = useState<'google' | 'cognito' | 'unknown'>('unknown')
  const [isLoading, setIsLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)
  const [isAuthenticated, setIsAuthenticated] = useState(false)
  const router = useRouter()

  useEffect(() => {
    const loadUserData = async () => {
      try {
        setIsLoading(true)
        
        // Check if user is authenticated
        const authStatus = await AuthService.isAuthenticated()
        setIsAuthenticated(authStatus)
        
        if (!authStatus) {
          router.push('/signin')
          return
        }

        // Get current user information
        const currentUser = await AuthService.getCurrentUser()
        setUser(currentUser)

        // Determine authentication method
        const method = await AuthService.getAuthenticationMethod()
        setAuthMethod(method)
        
      } catch (err) {
        console.error('Failed to load user data:', err)
        setError('Failed to load account information')
      } finally {
        setIsLoading(false)
      }
    }

    loadUserData()
  }, [router])

  const handleLogout = async () => {
    try {
      await AuthService.logoutAll()
      router.push('/signin')
    } catch (error) {
      console.error('Logout failed:', error)
      router.push('/signin')
    }
  }

  if (isLoading) {
    return (
      <div className="container mx-auto p-6 max-w-4xl">
        <div className="space-y-6">
          <div className="space-y-2">
            <Skeleton className="h-8 w-48" />
            <Skeleton className="h-4 w-96" />
          </div>
          <Card>
            <CardHeader>
              <div className="flex items-center space-x-4">
                <Skeleton className="h-16 w-16 rounded-full" />
                <div className="space-y-2">
                  <Skeleton className="h-6 w-48" />
                  <Skeleton className="h-4 w-64" />
                </div>
              </div>
            </CardHeader>
            <CardContent className="space-y-4">
              <Skeleton className="h-4 w-full" />
              <Skeleton className="h-4 w-3/4" />
              <Skeleton className="h-4 w-1/2" />
            </CardContent>
          </Card>
        </div>
      </div>
    )
  }

  if (error) {
    return (
      <div className="container mx-auto p-6 max-w-4xl">
        <Card>
          <CardContent className="flex items-center space-x-2 p-6">
            <AlertCircle className="h-5 w-5 text-destructive" />
            <span className="text-destructive">{error}</span>
          </CardContent>
        </Card>
      </div>
    )
  }

  if (!user) {
    return (
      <div className="container mx-auto p-6 max-w-4xl">
        <Card>
          <CardContent className="flex items-center space-x-2 p-6">
            <AlertCircle className="h-5 w-5 text-muted-foreground" />
            <span className="text-muted-foreground">No user information available</span>
          </CardContent>
        </Card>
      </div>
    )
  }

  const initials = (() => {
    if (user.given_name && user.family_name) {
      return `${user.given_name[0]}${user.family_name[0]}`.toUpperCase()
    }
    if (user.username && user.username.length > 0) {
      return user.username[0].toUpperCase()
    }
    return 'U'
  })()

  return (
    <div className="container mx-auto p-6 max-w-4xl space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">Account Settings</h1>
        <p className="text-muted-foreground">
          Manage your account information and preferences
        </p>
      </div>

      {/* Profile Information Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center space-x-2">
            <User className="h-5 w-5" />
            <span>Profile Information</span>
          </CardTitle>
          <CardDescription>
            Your account details and personal information
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Avatar and Basic Info */}
          <div className="flex items-center space-x-4">
            <Avatar className="h-16 w-16">
              <AvatarImage src={""} alt={user.username || 'User'} />
              <AvatarFallback className="text-lg font-semibold">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="space-y-1">
              <h3 className="text-xl font-semibold">
                {user.given_name && user.family_name 
                  ? `${user.given_name} ${user.family_name}` 
                  : user.username || 'Unknown User'}
              </h3>
              <p className="text-muted-foreground flex items-center space-x-1">
                <Mail className="h-4 w-4" />
                <span>{user.email || 'No email available'}</span>
              </p>
            </div>
          </div>

          <Separator />

          {/* Account Details */}
          <div className="grid gap-4 md:grid-cols-2">
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">Username</label>
              <p className="text-sm font-mono bg-muted p-2 rounded">{user.username || 'N/A'}</p>
            </div>
            
            <div className="space-y-2">
              <label className="text-sm font-medium text-muted-foreground">User ID</label>
              <p className="text-sm font-mono bg-muted p-2 rounded truncate" title={user.sub}>
                {user.sub}
              </p>
            </div>

            {user.given_name && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-muted-foreground">First Name</label>
                <p className="text-sm bg-muted p-2 rounded">{user.given_name}</p>
              </div>
            )}

            {user.family_name && (
              <div className="space-y-2">
                <label className="text-sm font-medium text-muted-foreground">Last Name</label>
                <p className="text-sm bg-muted p-2 rounded">{user.family_name}</p>
              </div>
            )}

            {user.email && (
              <div className="space-y-2 md:col-span-2">
                <label className="text-sm font-medium text-muted-foreground">Email Address</label>
                <div className="flex items-center space-x-2">
                  <p className="text-sm bg-muted p-2 rounded flex-1">{user.email}</p>
                  <Badge variant="secondary" className="flex items-center space-x-1">
                    <CheckCircle2 className="h-3 w-3" />
                    <span>Verified</span>
                  </Badge>
                </div>
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {/* Security Card */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center space-x-2">
            <Shield className="h-5 w-5" />
            <span>Security & Authentication</span>
          </CardTitle>
          <CardDescription>
            Manage your account security and authentication methods
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between p-4 border rounded-lg">
            <div className="flex items-center space-x-3">
              <Shield className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="font-medium">Authentication Method</p>
                <p className="text-sm text-muted-foreground">
                  {authMethod === 'google' ? 'Google OAuth' : 
                   authMethod === 'cognito' ? 'Email & Password' : 'Unknown'}
                </p>
              </div>
            </div>
            <Badge variant={authMethod === 'google' ? 'default' : 'secondary'}>
              {authMethod === 'google' ? 'Google' : 
               authMethod === 'cognito' ? 'Cognito' : 'Unknown'}
            </Badge>
          </div>

          <div className="flex items-center justify-between p-4 border rounded-lg">
            <div className="flex items-center space-x-3">
              <Key className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="font-medium">Account Status</p>
                <p className="text-sm text-muted-foreground">Your account is active and secure</p>
              </div>
            </div>
            <Badge variant="secondary" className="flex items-center space-x-1">
              <CheckCircle2 className="h-3 w-3" />
              <span>Active</span>
            </Badge>
          </div>

          <div className="flex items-center justify-between p-4 border rounded-lg">
            <div className="flex items-center space-x-3">
              <Clock className="h-5 w-5 text-muted-foreground" />
              <div>
                <p className="font-medium">Session Management</p>
                <p className="text-sm text-muted-foreground">Manage your active sessions</p>
              </div>
            </div>
            <Button 
              variant="outline" 
              size="sm"
              onClick={handleLogout}
            >
              Sign Out
            </Button>
          </div>
        </CardContent>
      </Card>

      {/* Actions Card */}
      <Card>
        <CardHeader>
          <CardTitle>Account Actions</CardTitle>
          <CardDescription>
            Manage your account settings and preferences
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          <Button variant="outline" className="w-full justify-start">
            <Mail className="h-4 w-4 mr-2" />
            Update Email Preferences
          </Button>
          
          <Button variant="outline" className="w-full justify-start">
            <Shield className="h-4 w-4 mr-2" />
            Security Settings
          </Button>
          
          <Separator />
          
          <Button 
            variant="destructive" 
            className="w-full justify-start"
            onClick={handleLogout}
          >
            <AlertCircle className="h-4 w-4 mr-2" />
            Sign Out of All Devices
          </Button>
        </CardContent>
      </Card>
    </div>
  )
}
