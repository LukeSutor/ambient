"use client";

import { AppSidebar } from "@/components/app-sidebar";
import { SiteHeader } from "@/components/site-header";
import { SidebarInset, SidebarProvider } from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import { useRoleAccess } from "@/lib/role-access";
import { useSettings } from "@/lib/settings";
import type * as React from "react";

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Use role access
  useRoleAccess("/secondary");
  // Use root settings
  useSettings(true);

  return (
    <div className="relative h-screen w-full rounded-lg overflow-hidden border bg-white [--header-height:calc(--spacing(16))]">
      <SidebarProvider className="h-full">
        <SiteHeader includeCollapse includeMaximize />
        <div className="flex flex-1 h-full overflow-hidden">
          <AppSidebar />
          <SidebarInset className="flex flex-1 flex-col min-h-0 overflow-hidden">
            <main className="flex-1 overflow-y-auto">
              <div className="h-(--header-height) shrink-0" />
              <div className="p-4">{children}</div>
            </main>
            <Toaster richColors />
          </SidebarInset>
        </div>
      </SidebarProvider>
    </div>
  );
}
