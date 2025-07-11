"use client"

import * as React from "react"
import {
  LifeBuoy,
  Settings2,
  SquareTerminal,
  House,
  Code,
  Blocks,
  CheckSquare
} from "lucide-react"
import { useSidebar } from "@/components/ui/sidebar"
import { AuthService, CognitoUserInfo } from "@/lib/auth"

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

const data = {
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
      title: "Tasks",
      url: "/tasks",
      icon: CheckSquare
    },
    {
      title: "Integrations",
      url: "/integrations",
      icon: Blocks
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
  const [userInfo, setUserInfo] = React.useState<CognitoUserInfo | null>(null);
  const [isLoadingUser, setIsLoadingUser] = React.useState(true);

  // Fetch user information on mount
  React.useEffect(() => {
    const fetchUserInfo = async () => {
      try {
        const user = await AuthService.getCurrentUser();
        setUserInfo(user);
      } catch (error) {
        console.error('Failed to fetch user info:', error);
      } finally {
        setIsLoadingUser(false);
      }
    };

    fetchUserInfo();
  }, []);

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
        icon: Code,   // Use the Code icon
      });
    }
  }

  const { state } = useSidebar()

  // Create user object for NavUser component
  const user = userInfo ? {
    name: userInfo.given_name && userInfo.family_name 
      ? `${userInfo.given_name} ${userInfo.family_name}`
      : userInfo.username || 'User',
    email: userInfo.email || '',
    avatar: '/', // You can add avatar logic here
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
