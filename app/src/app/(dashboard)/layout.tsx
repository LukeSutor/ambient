"use client";

import * as React from "react";
import { usePathname, redirect } from "next/navigation";
import Link from "next/link";
import { invoke } from "@tauri-apps/api/core";
import { AppProvider } from "@/lib/providers";

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

// Helper function to capitalize strings
const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export default function DashboardLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const [isLoading, setIsLoading] = React.useState(true);
  const pathname = usePathname();

  React.useEffect(() => {
    async function checkAccess() {
      try {
        // Check authentication
        const { AuthService } = await import('@/lib/auth');
        const isAuthenticated = await AuthService.isAuthenticated();
        if (!isAuthenticated) {
          redirect('/signin');
          return;
        }

        // Check setup completion
        const isSetupComplete = await invoke<boolean>("check_setup_complete");
        if (!isSetupComplete) {
          redirect('/setup');
          return;
        }

        setIsLoading(false);
      } catch (error) {
        console.error('Access check failed:', error);
        redirect('/signin');
      }
    }

    checkAccess();
  }, []);

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

  if (isLoading) {
    return (
      <SidebarProvider>
        {/* Skeleton Sidebar */}
        <div className="w-64 bg-white border-r border-gray-200 h-screen flex flex-col animate-pulse">
          {/* Sidebar Header */}
          <div className="p-4 border-b border-gray-200">
            <div className="h-8 bg-gray-200 rounded w-3/4"></div>
          </div>
          
          {/* Navigation Items */}
          <div className="flex-1 p-4 space-y-3">
            <div className="h-10 bg-gray-200 rounded"></div>
            <div className="h-10 bg-gray-200 rounded"></div>
            <div className="h-10 bg-gray-200 rounded"></div>
            <div className="h-10 bg-gray-200 rounded"></div>
            <div className="h-10 bg-gray-200 rounded"></div>
          </div>
          
          {/* Sidebar Footer */}
          <div className="p-4 border-t border-gray-200 space-y-2">
            <div className="h-8 bg-gray-200 rounded"></div>
            <div className="h-8 bg-gray-200 rounded"></div>
          </div>
        </div>
        
        {/* Main Content Area */}
        <div className="flex-1 flex flex-col">
          {/* Header Skeleton */}
          <div className="h-16 border-b border-gray-200 flex items-center px-4 animate-pulse">
            <div className="h-6 bg-gray-200 rounded w-8 mr-4"></div>
            <div className="h-4 w-px bg-gray-200 mr-4"></div>
            <div className="flex items-center space-x-2">
              <div className="h-4 bg-gray-200 rounded w-12"></div>
              <div className="h-4 bg-gray-200 rounded w-2"></div>
              <div className="h-4 bg-gray-200 rounded w-20"></div>
            </div>
          </div>
          
          {/* Main Content Skeleton */}
          <div className="flex-1 p-4 animate-pulse space-y-6">
            {/* Page Title */}
            <div className="h-8 bg-gray-200 rounded w-1/3"></div>
            
            {/* Content Cards */}
            <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
              <div className="bg-gray-100 p-6 rounded-lg space-y-3">
                <div className="h-6 bg-gray-200 rounded w-3/4"></div>
                <div className="h-4 bg-gray-200 rounded w-full"></div>
                <div className="h-4 bg-gray-200 rounded w-2/3"></div>
              </div>
              <div className="bg-gray-100 p-6 rounded-lg space-y-3">
                <div className="h-6 bg-gray-200 rounded w-3/4"></div>
                <div className="h-4 bg-gray-200 rounded w-full"></div>
                <div className="h-4 bg-gray-200 rounded w-2/3"></div>
              </div>
              <div className="bg-gray-100 p-6 rounded-lg space-y-3">
                <div className="h-6 bg-gray-200 rounded w-3/4"></div>
                <div className="h-4 bg-gray-200 rounded w-full"></div>
                <div className="h-4 bg-gray-200 rounded w-2/3"></div>
              </div>
            </div>
            
            {/* Large Content Block */}
            <div className="bg-gray-100 p-6 rounded-lg space-y-4">
              <div className="h-6 bg-gray-200 rounded w-1/4"></div>
              <div className="space-y-2">
                <div className="h-4 bg-gray-200 rounded w-full"></div>
                <div className="h-4 bg-gray-200 rounded w-5/6"></div>
                <div className="h-4 bg-gray-200 rounded w-4/5"></div>
              </div>
              <div className="flex space-x-2 pt-4">
                <div className="h-10 bg-gray-200 rounded w-24"></div>
                <div className="h-10 bg-gray-200 rounded w-20"></div>
              </div>
            </div>
          </div>
        </div>
      </SidebarProvider>
    );
  }

  return (
    <AppProvider>
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
    </AppProvider>
  );
}
