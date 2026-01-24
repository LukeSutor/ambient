"use client";

import { Button } from "@/components/ui/button";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, X } from "lucide-react";
import Image from "next/image";

interface SiteHeaderProps {
  includeMinimize?: boolean
}

export function SiteHeader({ includeMinimize }: SiteHeaderProps) {
  const closeWindow = async () => {
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.error("Failed to close window:", error);
    }
  };

  const minimizeWindow = async () => {
    try {
      await getCurrentWindow().minimize();
    } catch (error) {
      console.error("Failed to minimize window:", error);
    }
  };
  
  return (
    <header data-tauri-drag-region className="bg-background/80 backdrop-blur-md absolute top-0 left-0 right-0 z-50 border-b select-none flex justify-between items-center px-1 md:px-4 py-1 md:py-0 h-auto md:h-16">
        <div className="flex items-center">
          <Image 
            data-tauri-drag-region
            src="/logo.png" 
            alt="App Logo" 
            width={32} 
            height={32} 
            className="pointer-events-none"
          />
          <p className="ml-2 text-xl font-sora hidden md:block">ambient</p>
        </div>
        <div className="flex items-center">
          {includeMinimize && (
            <Button
              variant="ghost"
              size="icon"
              onClick={() => {
                void minimizeWindow();
              }}
            >
              <Minus className="!h-5 !w-5" />
            </Button>
          )}
          <Button
              className="hover:bg-gray-200"
              variant="ghost"
              size="icon"
              onClick={() => {
                void closeWindow();
              }}
            >
              <X className="!h-6 !w-6" />
            </Button>
        </div>
    </header>
  )
}