"use client";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CheckCircle, Loader2 } from "lucide-react";

interface SuccessCardProps {
  title: string;
  description: string;
  showRedirectMessage?: boolean;
  icon?: "check" | "loader";
}

export function SuccessCard({
  title,
  description,
  showRedirectMessage = true,
  icon = "check",
}: SuccessCardProps) {
  return (
    <div className="min-h-full flex items-center justify-center bg-background py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full">
        <Card className="text-center p-8">
          <CardHeader>
            <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-green-100 mb-4">
              {icon === "check" ? (
                <CheckCircle className="h-6 w-6 text-green-600" />
              ) : (
                <Loader2 className="h-6 w-6 text-green-600" />
              )}
            </div>
            <CardTitle className="text-2xl font-bold">{title}</CardTitle>
            <CardDescription>{description}</CardDescription>
          </CardHeader>
          {showRedirectMessage && (
            <CardContent className="text-center">
              <div className="animate-pulse text-sm text-gray-500">
                Redirecting to dashboard...
              </div>
            </CardContent>
          )}
        </Card>
      </div>
    </div>
  );
}
