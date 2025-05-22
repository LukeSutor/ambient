"use client"

import {
  Avatar,
  AvatarImage,
} from "@/components/ui/avatar"
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from "@/components/ui/sidebar"

export function NavLogo() {
  const { isMobile } = useSidebar()

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <SidebarMenuButton size="lg">
            <Avatar className="h-8 w-8 rounded-lg">
                <AvatarImage src="/logo.png" />
            </Avatar>
            <div className="flex flex-row items-center text-left text-xl leading-tight">
            <span className="truncate font-medium">cortical</span>
            </div>
        </SidebarMenuButton>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
