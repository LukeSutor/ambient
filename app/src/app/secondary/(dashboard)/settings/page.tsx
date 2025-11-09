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
  SelectLabel,
  SelectSeparator,
  SelectGroup,
} from "@/components/ui/select"
import { Badge } from "@/components/ui/badge"
import { toast } from "sonner"
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "@/lib/settings";
import { HudSizeOption, ModelSelection } from "@/types/settings";
import { Zap, Shield, Crown } from "lucide-react"

export default function Settings() {
    const { 
        settings, 
        isLoading, 
        setHudSize,
        setModelSelection,
    } = useSettings();

    const hudSize = settings?.hud_size ?? 'Normal';
    const modelSelection = settings?.model_selection ?? 'Local';

    async function handleHudSizeChange(value: string) {
        const newSize = value as HudSizeOption;
        
        try {
            await setHudSize(newSize, true);
            const displayName = newSize.charAt(0).toUpperCase() + newSize.slice(1);
            toast.success(`HUD size changed to ${displayName}`);
        } catch (error) {
            console.error("Failed to save HUD size setting:", error);
            toast.error("Failed to save setting");
        }
    }

    async function handleModelSelectionChange(value: string) {
        const newModel = value as ModelSelection;

        try {
            await setModelSelection(newModel);
            const displayName = newModel.charAt(0).toUpperCase() + newModel.slice(1);
            toast.success(`Model selection changed to ${displayName}`);
        } catch (error) {
            console.error("Failed to save model selection setting:", error);
            toast.error("Failed to save setting");
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

            {/* Model Settings */}
            <p className="text-xl font-semibold w-full pb-2">Model Settings</p>
            <div className="outline outline-gray-300 w-full rounded-md mb-6">
                <div className="flex flex-row justify-between p-4">
                    <div className="flex flex-col">
                        <p className="font-semibold text-sm">Model Selection</p>
                        <p className="text-sm text-gray-600">Choose the model to use for processing</p>
                    </div>
                    <Select 
                        value={modelSelection} 
                        onValueChange={handleModelSelectionChange}
                        disabled={isLoading}
                    >
                        <SelectTrigger className="">
                            <SelectValue placeholder="Select model">
                                {modelSelection === "Local" && (
                                    <div className="flex items-center gap-3">
                                        <div className="flex h-6 w-6 items-center justify-center rounded-full bg-green-100 text-green-600">
                                            <Shield className="h-4 w-4 m-1.5 text-green-600" />
                                        </div>
                                        <span className="font-medium">Local</span>
                                    </div>
                                )}
                                {modelSelection === "GptOss" && (
                                    <div className="flex items-center gap-3">
                                        <div className="flex h-6 w-6 items-center justify-center rounded-full bg-blue-100 text-blue-600">
                                            <Zap className="h-4 w-4 m-1.5 text-blue-600" />
                                        </div>
                                        <span className="font-medium">GPT OSS</span>
                                    </div>
                                )}
                                {modelSelection === "Gpt5" && (
                                    <div className="flex items-center gap-3">
                                        <div className="flex h-6 w-6 items-center justify-center rounded-full bg-gradient-to-r from-purple-500 to-pink-500">
                                            <Crown className="h-4 w-4 m-1.5 text-white" />
                                        </div>
                                        <span className="font-medium">GPT-5</span>
                                    </div>
                                )}
                                {!modelSelection && (
                                    <span className="text-muted-foreground">Select model</span>
                                )}
                            </SelectValue>
                        </SelectTrigger>
                        <SelectContent className="w-96">
                            <SelectGroup>
                                <SelectLabel className="text-xs font-medium text-muted-foreground px-2 py-1.5 flex items-center gap-2">
                                    <Zap className="h-3 w-3" />
                                    Available Models
                                </SelectLabel>
                                
                                <SelectItem value="Local" className="py-4 px-4 cursor-pointer h-auto min-h-[4rem]">
                                <div className="flex items-center justify-between w-full">
                                    <div className="flex items-center gap-3">
                                        <div className="flex items-center justify-center rounded-full bg-green-100">
                                            <Shield className="h-4 w-4 m-1.5 text-green-600" />
                                        </div>
                                        <div className="flex flex-col items-start">
                                            <div className="flex items-center gap-2">
                                                <span className="font-medium">Local</span>
                                                <Badge variant="outline" className="text-xs">Private</Badge>
                                            </div>
                                            <span className="text-xs text-muted-foreground text-left">
                                                Ultimate privacy. Runs on your device. No internet required.
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </SelectItem>

                            <SelectSeparator />

                            <SelectItem value="GptOss" className="py-4 px-4 cursor-pointer h-auto min-h-[4rem]">
                                <div className="flex items-center justify-between w-full">
                                    <div className="flex items-center gap-3">
                                        <div className="flex items-center justify-center rounded-full bg-blue-100">
                                            <Zap className="h-4 w-4 m-1.5 text-blue-600" />
                                        </div>
                                        <div className="flex flex-col items-start">
                                            <div className="flex items-center gap-2">
                                                <span className="font-medium">GPT OSS</span>
                                                <Badge variant="outline" className="text-xs">Enhanced</Badge>
                                            </div>
                                            <span className="text-xs text-muted-foreground text-left">
                                                More powerful. OpenAI's open-source model with advanced capabilities.
                                            </span>
                                        </div>
                                    </div>
                                </div>
                            </SelectItem>

                            <SelectSeparator />

                            <SelectItem value="Gpt5" className="py-4 px-4 cursor-pointer h-auto min-h-[4rem]">
                                <div className="flex items-center justify-between w-full">
                                    <div className="flex items-center gap-3">
                                        <div className="flex items-center justify-center rounded-full bg-gradient-to-r from-purple-500 to-pink-500">
                                            <Crown className="h-4 w-4 m-1.5 text-white" />
                                        </div>
                                        <div className="flex flex-col items-start">
                                            <div className="flex items-center gap-2">
                                                <span className="font-medium">GPT 5</span>
                                                <Badge variant="default" className="text-xs bg-gradient-to-r from-purple-500 to-pink-500 border-none">
                                                    Premium
                                                </Badge>
                                            </div>
                                            <span className="text-xs text-muted-foreground text-left">
                                                The latest and most advanced model from OpenAI.
                                            </span>
                                        </div>
                                    </div>
                                    <Button variant="outline" size="sm" className="h-6 mr-4 text-xs px-2 bg-gradient-to-r from-purple-50 to-pink-50 border-purple-200 hover:from-purple-100 hover:to-pink-100">
                                        Upgrade
                                    </Button>
                                </div>
                            </SelectItem>
                            </SelectGroup>
                        </SelectContent>
                    </Select>
                </div>
            </div>

            {/* Display Settings */}
            <p className="text-xl font-semibold w-full pb-2">Display Settings</p>
            <div className="outline outline-gray-300 w-full rounded-md mb-6">
                <div className="flex flex-row justify-between p-4">
                    <div className="flex flex-col">
                        <p className="font-semibold text-sm">Display Size</p>
                        <p className="text-sm text-gray-600">Choose the size of the chat display window</p>
                    </div>
                    <Select 
                        value={hudSize} 
                        onValueChange={handleHudSizeChange}
                        disabled={isLoading}
                    >
                        <SelectTrigger className="w-32">
                            <SelectValue placeholder="Select size" />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="Small">Small</SelectItem>
                            <SelectItem value="Normal">Normal</SelectItem>
                            <SelectItem value="Large">Large</SelectItem>
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