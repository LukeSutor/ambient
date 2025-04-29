"use client"

import * as React from "react"
import {
  Bot,
  Frame,
  LifeBuoy,
  Map,
  PieChart,
  Send,
  Settings2,
  SquareTerminal,
  House,
  Code
} from "lucide-react"

import { NavMain } from "@/components/nav-main"
import { NavSecondary } from "@/components/nav-secondary"
import { NavUser } from "@/components/nav-user"
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
} from "@/components/ui/sidebar"

const data = {
  user: {
    name: "shadcn",
    email: "m@example.com",
    avatar: "/avatars/shadcn.jpg",
  },
  navMain: [
    {
      title: "Dashboard",
      url: "/",
      icon: House
    },
    {
      title: "Activity",
      url: "/activity",
      icon: SquareTerminal,
      items: [
        {
          title: "Recent",
          url: "/activity/recent",
        },
        {
          title: "Recurring",
          url: "/activity/recurring",
        }
      ]
    },
    {
      title: "Status",
      url: "/status",
      icon: Bot
    }
  ],
  navSecondary: [
    {
      title: "Settings",
      url: "/settings",
      icon: Settings2,
    },
    {
      title: "Support",
      url: "/support",
      icon: LifeBuoy,
    }
  ]
}

export function AppSidebar({ ...props }: React.ComponentProps<typeof Sidebar>) {
  // Create a mutable copy of the nav items for this render
  const navItems = [...data.navMain];

  // Conditionally add the Debug item in development mode to the copy
  if (process.env.NODE_ENV === 'development') {
    // Optional: Check if it already exists (though copying usually prevents duplicates)
    const hasDebug = navItems.some(item => item.title === "Dev");
    if (!hasDebug) {
       navItems.push({
        title: "Dev",
        url: "/dev", // Point to your debug page route
        icon: Code,   // Use the Bug icon
      });
    }
  }

  return (
    <Sidebar variant="inset" {...props}>
      <SidebarHeader>
        <NavUser user={data.user} />
      </SidebarHeader>
      <SidebarContent>
        <NavMain items={navItems} />
        <NavSecondary items={data.navSecondary} className="mt-auto" />
      </SidebarContent>
    </Sidebar>
  )
}
