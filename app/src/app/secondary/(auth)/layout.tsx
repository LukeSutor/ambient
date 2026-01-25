"use client";

import { SiteHeader } from "@/components/site-header";
import { Toaster } from "@/components/ui/sonner";
import { useRoleAccess } from "@/lib/role-access";

export default function AuthLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Use role access
  useRoleAccess("/secondary");

  return (
    <div className="relative h-screen w-full rounded-lg overflow-hidden border bg-background flex items-center justify-center pt-16 pb-12 px-4 sm:px-6 lg:px-8">
      <SiteHeader includeMinimize />
      <div className="max-w-md w-full space-y-8 overflow-y-auto max-h-full">
        {children}
      </div>
      <Toaster richColors position="top-center" />
    </div>
  );
}
