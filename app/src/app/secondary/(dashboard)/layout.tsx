"use client";

import { AppSidebar } from "@/components/app-sidebar";
import { SiteHeader } from "@/components/site-header";
import {
  SidebarInset,
  SidebarProvider,
} from "@/components/ui/sidebar";
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
    <div className="relative h-full max-h-[800px] w-full rounded-lg overflow-hidden border bg-white [--header-height:calc(--spacing(16))]">
      <SidebarProvider>
        <SiteHeader />
        <div className="flex flex-1">
          <AppSidebar />
          <SidebarInset>
            <main className="flex flex-1 flex-col overflow-y-auto min-h-0 max-h-[800px]">
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
