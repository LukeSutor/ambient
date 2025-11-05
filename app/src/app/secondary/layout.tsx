"use client";

import * as React from "react";
import { usePathname } from "next/navigation";
import Link from "next/link";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { AppSidebar } from "@/components/app-sidebar";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb";
import { Separator } from "@/components/ui/separator";
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
    <div className="relative h-full w-1/2 rounded-xl overflow-hidden bg-black">
      {/* Top drag and close area */}
      <div data-tauri-drag-region className="absolute top-0 right-0 left-0 flex justify-end items-center bg-black rounded-lg border-b">
        <Button className="mr-4 hover:bg-gray-200" variant="ghost" size="icon" onClick={handleClose}>
          <X className="!h-6 !w-6" />
        </Button>
      </div>
      <SidebarProvider>
        <AppSidebar />
        <SidebarInset>
          <header className="flex h-16 shrink-0 items-center gap-2 border-b">
            <div className="flex items-center gap-2 px-4">
              <SidebarTrigger className="-ml-1" />
              <Separator
                orientation="vertical"
                className="mr-2 data-[orientation=vertical]:h-4"
              />
              <Breadcrumb>
                <BreadcrumbList>
                  {/* Add Home breadcrumb */}
                  <BreadcrumbItem>
                    {breadcrumbItems.length > 0 ? (
                      <BreadcrumbLink asChild>
                        <Link href="/">Home</Link>
                      </BreadcrumbLink>
                    ) : (
                      <BreadcrumbPage>Home</BreadcrumbPage>
                    )}
                  </BreadcrumbItem>
                  {/* Render dynamic breadcrumbs */}
                  {breadcrumbItems.map((item) => (
                    <React.Fragment key={item.href}>
                      <BreadcrumbSeparator />
                      <BreadcrumbItem>
                        {item.isLast ? (
                          <BreadcrumbPage>{item.title}</BreadcrumbPage>
                        ) : (
                          <BreadcrumbLink asChild>
                            <Link href={item.href}>{item.title}</Link>
                          </BreadcrumbLink>
                        )}
                      </BreadcrumbItem>
                    </React.Fragment>
                  ))}
                </BreadcrumbList>
              </Breadcrumb>
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
