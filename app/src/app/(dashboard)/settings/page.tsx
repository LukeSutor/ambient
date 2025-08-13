"use client"

import { Button } from "@/components/ui/button"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogClose,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { toast } from "sonner"
import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect } from "react";
import { SettingsService, HudSizeOption } from "@/lib/settings-service";

export default function Settings() {
    const [hudSize, setHudSize] = useState<HudSizeOption>(HudSizeOption.Normal);
    const [isLoadingSettings, setIsLoadingSettings] = useState(true);

    // Load current HUD size setting on component mount
    useEffect(() => {
        const loadCurrentSettings = async () => {
            try {
                const currentSize = await SettingsService.getHudSize();
                setHudSize(currentSize);
            } catch (error) {
                console.error("Failed to load HUD size setting:", error);
                toast.error("Failed to load settings");
            } finally {
                setIsLoadingSettings(false);
            }
        };

        loadCurrentSettings();
    }, []);

    async function handleHudSizeChange(value: string) {
        const newSize = value as HudSizeOption;
        setHudSize(newSize);
        
        try {
            // Let the HUD component handle the window resize via event listener
            await SettingsService.setHudSize(newSize);
            const displayName = newSize.charAt(0).toUpperCase() + newSize.slice(1);
            toast.success(`HUD size changed to ${displayName}`);
        } catch (error) {
            console.error("Failed to save HUD size setting:", error);
            toast.error("Failed to save setting");
            // Revert to previous value on error
            const previousSize = await SettingsService.getHudSize();
            setHudSize(previousSize);
        }
    }

    async function handleReset() {
        console.log("Attempting to reset database...");
        try {
            await invoke('reset_database');
            console.log("Database reset successfully!");
            toast.success('Database reset successful')
        } catch (error) {
            console.error("Failed to reset database:", error);
            toast.error('Database reset not successful')
        }
    }


    return (
        <div className="relative flex flex-col items-center justify-center p-4 max-w-2xl w-full mx-auto">
            {/* Display Settings */}
            <p className="text-xl font-semibold w-full pb-2">Display Settings</p>
            <div className="outline outline-gray-300 w-full rounded-md mb-6">
                <div className="flex flex-row justify-between p-4">
                    <div className="flex flex-col">
                        <p className="font-semibold text-sm">Display Size</p>
                        <p className="text-sm text-gray-600">Choose the size of the floating display window</p>
                    </div>
                    <Select 
                        value={hudSize} 
                        onValueChange={handleHudSizeChange}
                        disabled={isLoadingSettings}
                    >
                        <SelectTrigger className="w-32">
                            <SelectValue placeholder="Select size" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value={HudSizeOption.Small}>Small</SelectItem>
                            <SelectItem value={HudSizeOption.Normal}>Normal</SelectItem>
                            <SelectItem value={HudSizeOption.Large}>Large</SelectItem>
                        </SelectContent>
                    </Select>
                </div>
            </div>

            {/* Danger zone */}
            <p className="text-xl font-semibold w-full pb-2">Danger Zone</p>
            <div className="outline outline-red-500 w-full rounded-md">
                <Dialog>
                    <div className="flex flex-row justify-between p-4">
                        <div className="flex flex-col">
                            <p className="font-semibold text-sm">Delete personal data</p>
                            <p className="text-sm">This action is not reversible. Please be certain.</p>
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