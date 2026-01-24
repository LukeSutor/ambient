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
    // @ts-expect-error BigInt does not have a toJSON method by default, so we add one for JSON serialization
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
          {/* Force transparent background for this window on first paint */}
          <style
            /* biome-ignore lint/security/noDangerouslySetInnerHtml: Need to force transparent background for Tauri window */
            dangerouslySetInnerHTML={{
              __html:
                "html,body{background:transparent!important;background-color:transparent!important;}",
            }}
          />
          {children}
        </body>
      </html>
    </AppProvider>
  );
}
