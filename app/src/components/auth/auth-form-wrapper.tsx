"use client";

import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import type { ReactNode } from "react";

type AuthVariant = "secondary" | "hud";

interface AuthFormWrapperProps {
  title: string;
  description?: string;
  children: ReactNode;
  footer?: ReactNode;
  variant?: AuthVariant;
}

export function AuthFormWrapper({
  title,
  description,
  children,
  footer,
  variant = "secondary",
}: AuthFormWrapperProps) {
  if (variant === "hud") {
    return (
      <Card className="relative w-full pt-16 border-none shadow-none rounded-none">
        <CardHeader className="text-center">
          <CardTitle className="text-2xl font-bold">{title}</CardTitle>
          {description && <CardDescription>{description}</CardDescription>}
        </CardHeader>
        <CardContent>{children}</CardContent>
        {footer}
      </Card>
    );
  }

  return (
    <div className="min-h-full flex items-center justify-center bg-background py-12 px-4 sm:px-6 lg:px-8">
      <div className="max-w-md w-full space-y-8">
        <Card className="w-full">
          <CardHeader className="text-center">
            <CardTitle className="text-3xl font-bold">{title}</CardTitle>
            {description && <CardDescription>{description}</CardDescription>}
          </CardHeader>
          <CardContent>{children}</CardContent>
        </Card>
        {footer}
      </div>
    </div>
  );
}
