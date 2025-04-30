"use client"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogClose,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import { toast } from "sonner"
import { Toaster } from "@/components/ui/sonner"
import { invoke } from "@tauri-apps/api/core";

export default function Settings() {

    async function handleReset() {
        console.log("Attempting to reset database...");
        try {
            await invoke('reset_database');
            console.log("Database reset successfully!");
            toast("Data reset", {
                description: "Your data has been successfully removed."
            })
        } catch (error) {
            console.error("Failed to reset database:", error);
            toast("Error", {
                description: "Something went wrong, please try again later."
            })
        }
    }


    return (
        <div className="relative flex flex-col items-center justify-center p-4">
            {/* Danger zone */}
            <p className="text-xl font-semibold w-full pb-2">Danger Zone</p>
            <div className="outline outline-red-500 w-full max-w-xl rounded-md">
                <Toaster />
                <Dialog>
                    <div className="flex flex-row justify-between p-4">
                        <div className="flex flex-col">
                            <p className="font-semibold text-sm">Delete personal data</p>
                            <p className="text-sm">This action is not reversible, please be certain.</p>
                        </div>
                        <DialogTrigger asChild>
                            <Button variant="outline" className="text-red-500 hover:text-red-600 font-semibold">Reset</Button>
                        </DialogTrigger>
                        <DialogContent className="sm:max-w-[425px]">
                        <DialogHeader>
                            <DialogTitle>Are you sure?</DialogTitle>
                            <DialogDescription>
                                Once you delete your data, you won't be able to get it back!
                            </DialogDescription>
                        </DialogHeader>
                        {/* Removed type="submit" as it's not submitting a form */}
                        <DialogClose asChild>
                            <Button variant="destructive" onClick={handleReset}>Delete data</Button>
                        </DialogClose>
                    </DialogContent>
                    </div>
                </Dialog>
            </div>
        </div>
    );
}