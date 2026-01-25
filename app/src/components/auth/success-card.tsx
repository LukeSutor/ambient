"use client";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { CheckCircle, Loader2 } from "lucide-react";

type AuthVariant = "secondary" | "hud";

interface SuccessCardProps {
  title: string;
  description: string;
  showRedirectMessage?: boolean;
  icon?: "check" | "loader";
  variant?: AuthVariant;
}

export function SuccessCard({
  title,
  description,
  showRedirectMessage = true,
  icon = "check",
  variant = "secondary",
}: SuccessCardProps) {
  const cardContent = (
    <Card className={`text-center ${variant === "hud" ? "relative w-full pt-12 p-6" : "p-8"}`}>
      <CardHeader>
        <div className="mx-auto flex items-center justify-center h-12 w-12 rounded-full bg-green-100 mb-4">
          {icon === "check" ? (
            <CheckCircle className="h-6 w-6 text-green-600" />
          ) : (
            <Loader2 className="h-6 w-6 text-green-600" />
          )}
        </div>
        <CardTitle className={`${variant === "hud" ? "text-xl" : "text-2xl"} font-bold`}>
          {title}
        </CardTitle>
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
  );

  if (variant === "hud") {
    return cardContent;
  }

  return (
    <div className="min-h-full flex items-center justify-center bg-background py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full">
        {cardContent}
      </div>
    </div>
  );
}
