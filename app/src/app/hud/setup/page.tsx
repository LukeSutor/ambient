"use client";

import { useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useRouter } from 'next/navigation';
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { toast } from "sonner";
import { Toaster } from "@/components/ui/sonner";
import { Loader2, X } from "lucide-react";
import { useWindows } from "@/lib/windows/useWindows";

// Define the structure of the event payloads based on Rust code
interface DownloadStartedPayload {
  id: number;
  contentLength: number;
}

interface DownloadProgressPayload {
  id: number;
  totalProgress: number;
}

interface DownloadFinishedPayload {
  id: number;
}

// Helper function for formatting bytes
function formatBytes(bytes: number, decimals = 1): string {
  if (!+bytes) return '0 Bytes'

  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ['Bytes', 'KB', 'MB', 'GB']

  const i = Math.floor(Math.log(bytes) / Math.log(k))

  if (i === 3 && (bytes / Math.pow(k, i)) < 10) {
      return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
  }

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`
}

export default function SetupPage() {
  const router = useRouter();
  const [isSettingUp, setIsSettingUp] = useState(false);
  const [overallStatus, setOverallStatus] = useState("");

  // VLM Download specific state
  const [vlmDownloadId, setVlmDownloadId] = useState<number | null>(null);
  const [vlmTotalSize, setVlmTotalSize] = useState(0);
  const [vlmCurrentProgress, setVlmCurrentProgress] = useState(0);

  // Windows state
  const { closeHUD } = useWindows();

  // Function to start the setup process
  const handleStartSetup = useCallback(async () => {
    setIsSettingUp(true);
    setOverallStatus("Starting setup...");
    setVlmDownloadId(null);
    setVlmTotalSize(0);
    setVlmCurrentProgress(0);

    const listeners: UnlistenFn[] = [];

    try {
      // Setup listeners
      listeners.push(await listen<DownloadStartedPayload>('download-started', (event) => {
        console.log("Download Started:", event.payload);
        setVlmDownloadId(event.payload.id);
        setVlmTotalSize(event.payload.contentLength);
        setVlmCurrentProgress(0);
        setOverallStatus(`Downloading Model ${event.payload.id}`);
      }));

      listeners.push(await listen<DownloadProgressPayload>('download-progress', (event) => {
        setVlmDownloadId(currentId => {
            if (event.payload.id === currentId) {
                setVlmCurrentProgress(event.payload.totalProgress);
            }
            return currentId;
        });
      }));

      listeners.push(await listen<DownloadFinishedPayload>('download-finished', (event) => {
        console.log("Download Finished:", event.payload);
        if (event.payload.id === 2) {
            setOverallStatus("Finalizing");
            setVlmDownloadId(null);
            setVlmTotalSize(0);
            setVlmCurrentProgress(0);
        } else if (event.payload.id === 1) {
             setOverallStatus("Model 1 complete. Starting model 2.");
        }
      }));

      // Invoke the setup command
      const result = await invoke<string>("setup");
      console.log("[SetupPage] Setup command finished:", result);
      setOverallStatus("Setup completed successfully!");
      toast.success("Setup completed successfully!");

      // Redirect to HUD dashboard after successful setup
      router.push('/hud');

    } catch (err) {
      console.error("[SetupPage] Setup failed:", err);
      const errorMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'An unknown error occurred');
      toast.error(`Setup failed: ${errorMsg}`);
      setOverallStatus("Setup process encountered an error.");
    } finally {
      if (!overallStatus.includes("successfully")) {
          setIsSettingUp(false);
      }
      listeners.forEach(unlisten => unlisten());
      console.log("[SetupPage] Event listeners cleaned up.");
    }
  }, [router, overallStatus]);

  const progressPercent = vlmTotalSize > 0 ? (vlmCurrentProgress / vlmTotalSize) * 100 : 0;

  return (
    <div className="h-full w-full">
      <Toaster richColors position="top-center" />
      
      {/* Setup Card */}
      <Card className="relative w-full pt-12">
        {/* Drag area and close button */}
        <div data-tauri-drag-region className="fixed top-0 right-0 left-0 flex justify-end py-1 pr-1 items-center border-b">
          <Button className="hover:bg-gray-200" variant="ghost" size="icon" onClick={closeHUD}>
            <X className="!h-6 !w-6" />
          </Button>
        </div>

        <CardHeader className="text-center pt-2">
          <CardTitle className="text-2xl font-bold">Application Setup Required</CardTitle>
          <CardDescription>
            Essential models need to be downloaded before using the application.
            This might take some time depending on your internet connection. The total download size is approximately 0.7 GB.
          </CardDescription>
        </CardHeader>
        
        <CardContent className="space-y-4">
          {overallStatus !== "" ? 
          (<div className="text-sm text-muted-foreground">
            {overallStatus}
          </div>) : (
            <div className="h-[20px]" />
          )}

          {/* VLM Progress Display */}
          {isSettingUp && vlmDownloadId !== null && (
            <div className="space-y-2 pt-2">
              <div className="flex justify-between text-xs font-medium text-foreground">
                <span>
                  {`Model ${vlmDownloadId}`} ({formatBytes(vlmCurrentProgress)} / {formatBytes(vlmTotalSize)})
                </span>
                <span>{progressPercent.toFixed(0)}%</span>
              </div>
              <Progress value={progressPercent} className="w-full h-2" />
            </div>
          )}
          
          {/* Embedding Model Status Display */}
           {isSettingUp && vlmDownloadId === null && overallStatus.includes("Finalizing") && (
             <div className="space-y-2 pt-2 flex flex-row items-center justify-center">
                <div className="animate-spin">
                  <Loader2 />
                </div>
             </div>
           )}
        </CardContent>
        
        <CardFooter>
          {!isSettingUp && (
            <Button onClick={handleStartSetup} className="w-full h-11 text-base font-medium">
              Start Setup
            </Button>
          )}
           {isSettingUp && (
             <Button className="w-full h-11 text-base font-medium" disabled={true}>
               Setting Up...
             </Button>
           )}
        </CardFooter>
      </Card>
    </div>
  );
}
