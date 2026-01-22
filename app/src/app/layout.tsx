"use client";

import { AppProvider } from "@/lib/providers";
import { Geist, Geist_Mono, Sora } from "next/font/google";
import * as React from "react";

import "@/styles/globals.css";
import "katex/dist/katex.min.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

const sora = Sora({
  variable: "--font-sora",
  subsets: ["latin"],
  weight: ["400", "700", "800"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  // Add BigInt serialization support
  React.useEffect(() => {
    // @ts-ignore
    BigInt.prototype.toJSON = function () {
      return this.toString();
    };
  }, []);

  return (
    <AppProvider>
      <html lang="en">
        <body
          className={`${geistSans.variable} ${geistMono.variable} ${sora.variable} antialiased`}
        >
          {children}
        </body>
      </html>
    </AppProvider>
  );
}
