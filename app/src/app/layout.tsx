"use client";

import * as React from "react";
import { Geist, Geist_Mono } from "next/font/google";
import { usePathname } from "next/navigation";
import Link from "next/link";

import "@/styles/globals.css";

import { AppSidebar } from "@/components/app-sidebar"
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "@/components/ui/breadcrumb"
import { Separator } from "@/components/ui/separator"
import {
  SidebarInset,
  SidebarProvider,
  SidebarTrigger,
} from "@/components/ui/sidebar"

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
  const pathname = usePathname();

  // Generate breadcrumb items based on the pathname
  const pathSegments = pathname.split('/').filter(Boolean); // Split and remove empty strings
  const breadcrumbItems = pathSegments.map((segment, index) => {
    const href = '/' + pathSegments.slice(0, index + 1).join('/');
    // Decode URI component for segments like %20 and capitalize
    const title = capitalize(decodeURIComponent(segment).replace(/-/g, ' '));
    const isLast = index === pathSegments.length - 1;
    return { href, title, isLast };
  });

  return (
    <html lang="en">
      <body
        className={`${geistSans.variable} ${geistMono.variable} antialiased`}
      >
        <SidebarProvider>
          <AppSidebar />
          <SidebarInset>
            <header className="flex h-16 shrink-0 items-center gap-2">
              <div className="flex items-center gap-2 px-4">
                <SidebarTrigger className="-ml-1" />
                <Separator
                  orientation="vertical"
                  className="mr-2 data-[orientation=vertical]:h-4"
                />
                <Breadcrumb>
                  <BreadcrumbList>
                    {breadcrumbItems.map((item, index) => (
                      <React.Fragment key={item.href}>
                        {/* Add separator only if it's not the first item */}
                        {index > 0 && <BreadcrumbSeparator />}
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
                    {/* Handle case where there are no segments (root path) */}
                    {breadcrumbItems.length === 0 && (
                       <BreadcrumbItem>
                         <BreadcrumbPage>Home</BreadcrumbPage>
                       </BreadcrumbItem>
                    )}
                  </BreadcrumbList>
                </Breadcrumb>
              </div>
            </header>
            {children}
          </SidebarInset>
        </SidebarProvider>
      </body>
    </html>
  );
}
