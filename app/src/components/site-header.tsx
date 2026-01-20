"use client"

import { Minus, X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { SidebarTrigger, useSidebar } from "@/components/ui/sidebar"

type SiteHeaderProps = {
  handleClose?: () => Promise<void>;
  handleMinimize?: () => Promise<void>;
}

export function SiteHeader({ handleClose, handleMinimize }: SiteHeaderProps) {
  return (
    <header className="bg-background sticky top-0 z-50 flex w-full items-center border-b h-16 shrink-0 gap-2">
      <div className="flex items-center gap-2 px-4 w-full h-full">
        <SidebarTrigger className="-ml-1" />
        <div data-tauri-drag-region className="w-full h-full flex justify-end items-center">
          {/* Window minimize button */}
          <Button variant="ghost" size="icon" onClick={handleMinimize}>
            <Minus className="!h-5 !w-5" />
          </Button>
          {/* Window close button */}
          <Button variant="ghost" size="icon" onClick={handleClose}>
            <X className="!h-5 !w-5" />
          </Button>
        </div>
      </div>
    </header>
  )
}
