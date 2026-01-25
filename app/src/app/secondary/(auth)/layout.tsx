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
    <div className="relative h-full max-h-[800px] w-full rounded-xl overflow-hidden border bg-gray-50 flex items-center justify-center py-12 px-4 sm:px-6 lg:px-8">
      <SiteHeader includeMinimize />
      <div className="max-w-md bg-black w-full space-y-8">{children}</div>
      <Toaster richColors position="top-center" />
    </div>
  );
}
