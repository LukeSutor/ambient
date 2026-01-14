"use client"

import * as React from "react"
import {
  LifeBuoy,
  Settings2,
  SquareTerminal,
  House,
  Code,
  NotebookPen,
  CheckSquare
} from "lucide-react"
import { useSidebar } from "@/components/ui/sidebar"

import { Separator } from "@/components/ui/separator"
import { NavMain } from "@/components/nav-main"
import { NavSecondary } from "@/components/nav-secondary"
import { NavUser } from "@/components/nav-user"
import { NavLogo } from "@/components/nav-logo"
import {
  Sidebar,
  SidebarContent,
  SidebarHeader,
  SidebarFooter
} from "@/components/ui/sidebar"
import { useRoleAccess } from "@/lib/role-access"

const data = {
  navMain: [
    {
      title: "Dashboard",
      url: "/secondary",
      icon: House
    },
    {
      title: "Activity",
      url: "/secondary/activity",
      icon: SquareTerminal,
      items: [
        {
          title: "Recent",
          url: "/secondary/activity/recent",
        },
        {
          title: "Recurring",
          url: "/secondary/activity/recurring",
        }
      ]
    },
    {
      title: "Memories",
      url: "/secondary/memories",
      icon: NotebookPen
    }
  ],
  navSecondary: [
    {
      title: "Settings",
      url: "/secondary/settings",
      icon: Settings2,
    },
    {
      title: "Support",
      url: "/secondary/support",
      icon: LifeBuoy,
    }
  ]
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  const { userInfo } = useRoleAccess();

  // Create a mutable copy of the nav items for this render
  const navItems = [...data.navMain];

  // Conditionally add the Debug item in development mode to the copy
  if (process.env.NODE_ENV === 'development') {
    // Optional: Check if it already exists (though copying usually prevents duplicates)
    const hasDebug = navItems.some(item => item.title === "Dev");
    if (!hasDebug) {
       navItems.push({
        title: "Dev",
        url: "/secondary/dev", // Point to your debug page route
        icon: Code,   // Use the Code icon
      });
    }
  }

  const { state } = useSidebar()

  // Create user object for NavUser component
  const user = userInfo ? {
    name: userInfo.full_name || userInfo.email?.split('@')[0] || 'User',
    email: userInfo.email || '',
    avatar: userInfo.avatar_url || '/',
  } : {
    name: 'User',
    email: '',
    avatar: '/',
  };

  return (
    <Sidebar collapsible="icon" variant="inset" {...props}>
      <SidebarHeader>
        <NavLogo />
      </SidebarHeader>
      <Separator className={state === "collapsed" ? "hidden" : "block"} />
      <SidebarContent>
        <NavMain items={navItems} />
        <NavSecondary items={data.navSecondary} className="mt-auto" />
      </SidebarContent>
      <SidebarFooter>
        <NavUser user={user} />
      </SidebarFooter>
    </Sidebar>
  )
}
