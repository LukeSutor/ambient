"use client";

import * as React from "react";
import { Geist, Geist_Mono } from "next/font/google";
import { AppProvider } from "@/lib/providers";

import "@/styles/globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Add BigInt serialization support
  React.useEffect(() => {
    // @ts-ignore
    BigInt.prototype.toJSON = function() {
      return this.toString();
    };
  }, []);
  return (
    <html lang="en" className="bg-transparent rounded-xl overflow-hidden w-1/2">
      <AppProvider>
        <body
          className={`${geistSans.variable} ${geistMono.variable} antialiased`}
        >
          {children}
        </body>
      </AppProvider>
    </html>
  );
}
