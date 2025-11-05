"use client";

import * as React from "react";
import { usePathname } from "next/navigation";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { AppSidebar } from "@/components/app-sidebar";
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";

// Helper function to capitalize strings
const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const pathname = usePathname();

  const handleClose = async () => {
    await invoke("close_secondary_window");
  }

  // Generate breadcrumbs
  const breadcrumbItems = React.useMemo(() => {
    const pathSegments = pathname.split('/').filter(Boolean);
    return pathSegments.map((segment, index) => {
      const href = '/' + pathSegments.slice(0, index + 1).join('/');
      const title = capitalize(decodeURIComponent(segment).replace(/-/g, ' '));
      const isLast = index === pathSegments.length - 1;
      return { href, title, isLast };
    });
  }, [pathname]);

  return (
    <div className="relative h-full max-h-[800px] w-full rounded-xl overflow-hidden border">
      {/* Force transparent background for this window on first paint */}
      <style
        // eslint-disable-next-line react/no-danger
        dangerouslySetInnerHTML={{
          __html:
            "html,body{background:transparent!important;background-color:transparent!important;}",
        }}
      />

      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <header className="flex h-16 shrink-0 items-center gap-2 border-b">
            <div className="flex items-center gap-2 px-4 w-full">
              <SidebarTrigger className="-ml-1" />
              <div data-tauri-drag-region className="w-full flex justify-end items-center">
                <Button variant="ghost" size="icon" onClick={handleClose}>
                  <X className="!h-6 !w-6" />
                </Button>
              </div>
            </div>
          </header>
          <main className="flex flex-1 flex-col p-4">
            {children}
          </main>
          <Toaster richColors />
        </SidebarInset>
      </SidebarProvider>
  </div>
  );
}
