"use client";

import { Button } from "@/components/ui/button";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Maximize, Minimize, Minus, X } from "lucide-react";
import Image from "next/image";
import { useEffect, useState } from "react";

interface SiteHeaderProps {
  includeCollapse?: boolean;
  includeMaximize?: boolean;
}

export function SiteHeader({
  includeCollapse,
  includeMaximize,
}: SiteHeaderProps) {
  const [isMaximized, setIsMaximized] = useState(false);

  useEffect(() => {
    if (!includeMaximize) return;

    const window = getCurrentWindow();

    const updateMaximized = async () => {
      const maximized = await window.isMaximized();
      setIsMaximized(maximized);
    };

    // Initial check
    void updateMaximized();

    // Listen for resize events which capture maximize/unmaximize
    const unlisten = window.onResized(() => {
      void updateMaximized();
    });

    return () => {
      void unlisten.then((fn) => {
        fn();
      });
    };
  }, [includeMaximize]);

  const minimizeWindow = async () => {
    try {
      await getCurrentWindow().minimize();
    } catch (error) {
      console.error("Failed to minimize window:", error);
    }
  };

  const maximizeWindow = async () => {
    try {
      const window = getCurrentWindow();
      if (isMaximized) {
        await window.unmaximize();
      } else {
        await window.maximize();
      }
    } catch (error) {
      console.error("Failed to toggle maximize window:", error);
    }
  };

  const closeWindow = async () => {
    try {
      await getCurrentWindow().close();
    } catch (error) {
      console.error("Failed to close window:", error);
    }
  };

  return (
    <header
      data-tauri-drag-region
      className="bg-background/80 backdrop-blur-md absolute top-0 left-0 right-0 z-50 border-b select-none flex justify-between items-center px-1 md:px-4 py-1 md:py-0 h-auto md:h-16"
    >
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
        {includeCollapse && (
          <Button
            variant="ghost"
            size="icon"
            onClick={() => {
              void minimizeWindow();
            }}
          >
            <Minus className="!h-6 !w-6" />
          </Button>
        )}
        {includeMaximize && (
          <Button
            variant="ghost"
            size="icon"
            onClick={() => {
              void maximizeWindow();
            }}
          >
            {isMaximized ? (
              <Minimize className="!h-6 !w-6" />
            ) : (
              <Maximize className="!h-6 !w-6" />
            )}
          </Button>
        )}
        <Button
          className="hover:bg-red-500 hover:text-white"
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
  );
}
