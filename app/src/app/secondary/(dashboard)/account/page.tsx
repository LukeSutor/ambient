"use client";

import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useRoleAccess } from "@/lib/role-access";
import { AlertCircle, Mail, User } from "lucide-react";
const googleLogo = "/google-logo.png";

export default function AccountPage() {
  // Auth state
  const { isHydrated, userInfo } = useRoleAccess();

  if (!isHydrated) {
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
    );
  }

  if (!userInfo) {
    return (
      <div className="container mx-auto p-6 max-w-4xl">
        <Card>
          <CardContent className="flex items-center space-x-2 p-6">
            <AlertCircle className="h-5 w-5 text-muted-foreground" />
            <span className="text-muted-foreground">
              No user information available
            </span>
          </CardContent>
        </Card>
      </div>
    );
  }

  const initials = (() => {
    if (userInfo.full_name) {
      const parts = userInfo.full_name.split(" ");
      if (parts.length >= 2)
        return `${parts[0][0]}${parts[parts.length - 1][0]}`.toUpperCase();
      return userInfo.full_name[0].toUpperCase();
    }
    return userInfo.email ? userInfo.email[0].toUpperCase() : "U";
  })();

  return (
    <div className="container mx-auto p-6 max-w-4xl space-y-6">
      {/* Header */}
      <div className="space-y-2">
        <h1 className="text-3xl font-bold tracking-tight">
          Account Information
        </h1>
        <p className="text-muted-foreground">Manage your account information</p>
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
              <AvatarImage
                src={userInfo.avatar_url ?? ""}
                alt={userInfo.email ?? "User"}
              />
              <AvatarFallback className="text-lg font-semibold">
                {initials}
              </AvatarFallback>
            </Avatar>
            <div className="space-y-1">
              <h3 className="text-xl font-semibold">
                {userInfo.full_name ?? userInfo.email ?? "Unknown User"}
                {!userInfo.full_name &&
                  userInfo.providers?.includes("google") && (
                    <Tooltip>
                      <TooltipTrigger>
                        <Badge
                          className="ml-2 h-7 w-7 rounded-full p-1"
                          variant="outline"
                        >
                          <img
                            src={googleLogo}
                            alt="Google Logo"
                            className="w-4 h-4"
                          />
                        </Badge>
                      </TooltipTrigger>
                      <TooltipContent>
                        <span>Authenticated via Google</span>
                      </TooltipContent>
                    </Tooltip>
                  )}
              </h3>
              {userInfo.full_name && (
                <p className="text-muted-foreground flex items-center space-x-1">
                  <Mail className="h-4 w-4" />
                  <span>{userInfo.email ?? "No email available"}</span>
                  {userInfo.providers?.includes("google") && (
                    <Tooltip>
                      <TooltipTrigger>
                        <Badge
                          className="ml-2 h-7 w-7 rounded-full p-1"
                          variant="outline"
                        >
                          <img
                            src={googleLogo}
                            alt="Google Logo"
                            className="w-4 h-4"
                          />
                        </Badge>
                      </TooltipTrigger>
                      <TooltipContent>
                        <span>Authenticated via Google</span>
                      </TooltipContent>
                    </Tooltip>
                  )}
                </p>
              )}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
