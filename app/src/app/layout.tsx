"use client";

import * as React from "react";
import { Geist, Geist_Mono } from "next/font/google";
import { usePathname } from "next/navigation";
import Link from "next/link";
import { invoke } from "@tauri-apps/api/core"; // Import invoke

import "@/styles/globals.css";

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
import SetupPage from "./setup/page"; // Import the SetupPage component
import { Toaster } from "@/components/ui/sonner"; // Import Toaster for potential layout-level errors

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

// Helper function to capitalize strings
const capitalize = (s: string) => s.charAt(0).toUpperCase() + s.slice(1);

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // --- Setup check state ---
  const [isLoadingSetupCheck, setIsLoadingSetupCheck] = React.useState(true);
  const [isSetupComplete, setIsSetupComplete] = React.useState(true);
  const [setupCheckError, setSetupCheckError] = React.useState<string | null>(null);

  // Always call usePathname (never conditionally)
  const pathname = usePathname();

  // Only check setup ONCE on mount
  React.useEffect(() => {
    const checkStatus = async () => {
      setIsLoadingSetupCheck(true);
      setSetupCheckError(null);
      try {
        const complete = await invoke<boolean>("check_setup_complete");
        setIsSetupComplete(complete);
      } catch (err) {
        const errorMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'An unknown error occurred');
        setSetupCheckError(`Failed to check setup status: ${errorMsg}. Please ensure the backend is running.`);
        setIsSetupComplete(false);
      } finally {
        setIsLoadingSetupCheck(false);
      }
    };
    checkStatus();
  }, []); // <-- only runs once

  const handleSetupComplete = React.useCallback(() => {
    setIsSetupComplete(true);
  }, []);

  // Only generate breadcrumbs if setup is complete
  const breadcrumbItems = React.useMemo(() => {
    if (!isSetupComplete) return [];
    const pathSegments = pathname.split('/').filter(Boolean);
    return pathSegments.map((segment, index) => {
      const href = '/' + pathSegments.slice(0, index + 1).join('/');
      const title = capitalize(decodeURIComponent(segment).replace(/-/g, ' '));
      const isLast = index === pathSegments.length - 1;
      return { href, title, isLast };
    });
  }, [pathname, isSetupComplete]);


  return (
    <html lang="en">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        {/* Render based on setup status */}
        {setupCheckError ? (
           <div className="flex h-full w-full items-center justify-center p-4 text-center text-red-600">
             <div>
                <h1 className="text-xl font-bold mb-2">Error</h1>
                <p>{setupCheckError}</p>
                <p className="mt-2 text-sm text-gray-500">You might need to restart the application.</p>
             </div>
             <Toaster richColors position="top-center" />
           </div>
        ) : !isSetupComplete ? (
          // Render only SetupPage if setup is not complete
          <SetupPage onSetupComplete={handleSetupComplete} />
        ) : (
          // Render the full layout if setup is complete
          <SidebarProvider>
            <AppSidebar />
            <SidebarInset>
              <header className="flex h-16 shrink-0 items-center gap-2 border-b"> {/* Added border */}
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
              <main className="flex flex-1 flex-col p-4"> {/* Added main wrapper */}
                {children}
              </main>
              <Toaster richColors /> {/* Toaster for the main app */}
            </SidebarInset>
          </SidebarProvider>
        )}
      </body>
    </html>
  );
}
