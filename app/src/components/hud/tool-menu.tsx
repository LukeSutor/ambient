"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InputGroupButton } from "@/components/ui/input-group";
import { useConversation } from "@/lib/conversations";
import {
  Bot,
  Globe,
  SquareDashedMousePointer,
  Wrench,
} from "lucide-react";
import { useCallback } from "react";

interface ToolMenuProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
}

export function ToolMenu({ onOpenChange, disabled }: ToolMenuProps) {
  const { conversationType, dispatchOCRCapture, toggleBrowserUse } =
    useConversation();
  const showToolsLabel = conversationType === "chat";

  const handleDispatchOCRCapture = useCallback(() => {
    void dispatchOCRCapture();
  }, [dispatchOCRCapture]);

  return (
    <DropdownMenu onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <InputGroupButton variant="ghost" disabled={disabled}>
          <Wrench className={showToolsLabel ? "mr-1" : ""} />
          {showToolsLabel && "Tools"}
        </InputGroupButton>
      </DropdownMenuTrigger>
      <DropdownMenuContent
        side="bottom"
        align="start"
        avoidCollisions={false}
        sideOffset={10}
        className="bg-white/60"
      >
        <DropdownMenuGroup>
          <DropdownMenuItem
            className="hover:bg-white/60"
            onClick={handleDispatchOCRCapture}
          >
            <SquareDashedMousePointer className="!w-4 !h-4 text-black shrink-0 mr-2" />
            <span className="text-black text-sm whitespace-nowrap">
              Capture Area
            </span>
          </DropdownMenuItem>
          <DropdownMenuItem
            className="hover:bg-white/60"
            onClick={toggleBrowserUse}
          >
            <Globe className="!w-4 !h-4 text-black shrink-0 mr-2" />
            <span className="text-black text-sm whitespace-nowrap">
              Browser Use
            </span>
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
