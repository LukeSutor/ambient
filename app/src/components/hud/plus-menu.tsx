"use client";

import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { InputGroupButton } from "@/components/ui/input-group";
import { useWindows } from "@/lib/windows/useWindows";
import { History, Paperclip, Plus, Settings2 } from "lucide-react";
import type React from "react";
import { useCallback } from "react";

interface PlusMenuProps {
  onOpenChange: (open: boolean) => void;
  disabled?: boolean;
  handleUploadFiles: (e: React.ChangeEvent<HTMLInputElement>) => void;
}

const ACCEPTED_FILE_TYPES = ".jpg, .jpeg, .png, .pdf";

export function PlusMenu({
  onOpenChange,
  disabled,
  handleUploadFiles,
}: PlusMenuProps) {
  const { toggleChatHistory, openSecondary } = useWindows();

  const handleFileUploadClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      e.preventDefault();
      e.stopPropagation();
      const input = e.currentTarget.querySelector("input");
      input?.click();
    },
    [],
  );

  const handleFileInputClick = useCallback(
    (e: React.MouseEvent<HTMLInputElement>) => {
      e.stopPropagation();
    },
    [],
  );

  return (
    <DropdownMenu onOpenChange={onOpenChange}>
      <DropdownMenuTrigger asChild>
        <InputGroupButton
          variant="outline"
          className="rounded-full bg-white/60 hover:bg-white/80"
          size="icon-xs"
          disabled={disabled}
        >
          <Plus />
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
        <DropdownMenuItem
          className="hover:bg-white/60 cursor-pointer"
          onClick={handleFileUploadClick}
        >
          <Paperclip className="!w-4 !h-4 text-black shrink-0 mr-2" />
          <span className="text-black text-sm whitespace-nowrap">
            Upload files
          </span>
          <input
            type="file"
            className="hidden"
            multiple
            accept={ACCEPTED_FILE_TYPES}
            onClick={handleFileInputClick}
            onChange={handleUploadFiles}
          />
        </DropdownMenuItem>
        <DropdownMenuSeparator className="w-11/12 mx-auto" />
        <DropdownMenuItem
          className="hover:bg-white/60"
          onClick={() => void toggleChatHistory()}
        >
          <History className="!w-4 !h-4 text-black shrink-0 mr-2" />
          <span className="text-black text-sm whitespace-nowrap">
            Previous Chats
          </span>
        </DropdownMenuItem>
        <DropdownMenuItem
          className="hover:bg-white/60"
          onClick={() => void openSecondary()}
        >
          <Settings2 className="!w-4 !h-4 text-black shrink-0 mr-2" />
          <span className="text-black text-sm whitespace-nowrap">
            Dashboard
          </span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
