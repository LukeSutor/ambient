"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuGroup,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InputGroupButton } from "@/components/ui/input-group";
import { MousePointerClick, SquareDashedMousePointer, Wrench } from "lucide-react";

interface ToolMenuProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
  conversationType: string;
  dispatchOCRCapture: () => void;
  toggleComputerUse: () => void;
}

export function ToolMenu({
  onOpenChange,
  disabled,
  conversationType,
  dispatchOCRCapture,
  toggleComputerUse,
}: ToolMenuProps) {
  const showToolsLabel = conversationType === "chat";

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
        alignOffset={-12}
        className="bg-white/60"
      >
        <DropdownMenuGroup>
          <DropdownMenuItem className="hover:bg-white/60" onClick={dispatchOCRCapture}>
            <SquareDashedMousePointer className="!w-4 !h-4 text-black shrink-0 mr-2" />
            <span className="text-black text-sm whitespace-nowrap">
              Capture Area
            </span>
          </DropdownMenuItem>
          <DropdownMenuItem className="hover:bg-white/60" onClick={toggleComputerUse}>
            <MousePointerClick className="!w-4 !h-4 text-black shrink-0 mr-2" />
            <span className="text-black text-sm whitespace-nowrap">
              Computer Use
            </span>
          </DropdownMenuItem>
        </DropdownMenuGroup>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
