"use client";

import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";

interface DangerZoneProps {
  onReset: () => void;
}

export function DangerZone({ onReset }: DangerZoneProps) {
  return (
    <Dialog>
      <div className="flex flex-row justify-between p-4">
        <div className="flex flex-col">
          <p className="font-semibold text-sm">Delete personal data</p>
          <p className="text-sm">
            This action is not reversible. Please be certain.
          </p>
        </div>
        <DialogTrigger asChild>
          <Button
            variant="outline"
            className="text-red-500 hover:text-red-600 font-semibold"
          >
            Reset
          </Button>
        </DialogTrigger>
        <DialogContent className="sm:max-w-[425px]">
          <DialogHeader>
            <DialogTitle>Are you sure?</DialogTitle>
            <DialogDescription>
              Once you delete your data, you won&apos;t be able to get it back!
            </DialogDescription>
          </DialogHeader>
          <DialogClose asChild>
            <Button variant="destructive" onClick={onReset}>
              Delete data
            </Button>
          </DialogClose>
        </DialogContent>
      </div>
    </Dialog>
  );
}
