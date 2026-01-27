"use client";

import { useSidebar } from "@/components/ui/sidebar";
import { cn } from "@/lib/utils";
import {
  Code,
  House,
  LifeBuoy,
  NotebookPen,
  Settings2,
  SquareTerminal,
} from "lucide-react";
import type * as React from "react";

import { NavMain } from "@/components/nav-main";
import { NavSecondary } from "@/components/nav-secondary";
import { NavUser } from "@/components/nav-user";
import { Separator } from "@/components/ui/separator";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarHeader,
} from "@/components/ui/sidebar";
import { useRoleAccess } from "@/lib/role-access";
import { NavHeader } from "./nav-header";

const data = {
  navMain: [
    {
      title: "Dashboard",
      url: "/secondary",
      icon: House,
    },
    {
      title: "Memories",
      url: "/secondary/memories",
      icon: NotebookPen,
    },
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
    },
  ],
};

export function AppSidebar({
  className,
  ...props
}: React.ComponentProps<typeof Sidebar>) {
  const { userInfo } = useRoleAccess();

  // Create a mutable copy of the nav items for this render
  const navItems = [...data.navMain];

  // Conditionally add the Debug item in development mode to the copy
  if (process.env.NODE_ENV === "development") {
    // Optional: Check if it already exists (though copying usually prevents duplicates)
    const hasDebug = navItems.some((item) => item.title === "Dev");
    if (!hasDebug) {
      navItems.push({
        title: "Dev",
        url: "/secondary/dev", // Point to your debug page route
        icon: Code, // Use the Code icon
      });
    }
  }

  const { state } = useSidebar();

  // Create user object for NavUser component
  const user = userInfo
    ? {
        name: userInfo.full_name ?? userInfo.email?.split("@")[0] ?? "User",
        email: userInfo.email ?? "",
        avatar: userInfo.avatar_url ?? "/",
      }
    : {
        name: "User",
        email: "",
        avatar: "/",
      };

  return (
    <Sidebar
      variant="floating"
      collapsible="icon"
      className={cn(
        "top-(--header-height) h-[calc(100svh-var(--header-height))]!",
        className,
      )}
      {...props}
    >
      <SidebarHeader>
        <NavHeader />
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
  );
}
