"use client";

import { useState, useEffect, useCallback } from "react";
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
import { toast } from "sonner"; // Import toast
import { Toaster } from "@/components/ui/sonner"; // Import Toaster
import { Loader2 } from "lucide-react";

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

// Helper function for formatting bytes (remains the same)
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

// Define props for the component - no longer needed
// interface SetupPageProps {
//   onSetupComplete: () => void;
// }

// Remove onSetupComplete prop
export default function SetupPage() {
  const router = useRouter();
  const [isSettingUp, setIsSettingUp] = useState(false);
  const [overallStatus, setOverallStatus] = useState("");

  // VLM Download specific state
  const [vlmDownloadId, setVlmDownloadId] = useState<number | null>(null);
  const [vlmTotalSize, setVlmTotalSize] = useState(0);
  const [vlmCurrentProgress, setVlmCurrentProgress] = useState(0);

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

      // Redirect to dashboard after successful setup
      router.push('/');

    } catch (err) {
      console.error("[SetupPage] Setup failed:", err);
      const errorMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'An unknown error occurred');
      // Use toast for error instead of Alert
      toast.error(`Setup failed: ${errorMsg}`);
      setOverallStatus("Setup process encountered an error.");
    } finally {
      // Set isSettingUp false only on error or completion (handled by onSetupComplete call)
      // If an error occurs, we might want the user to be able to retry
      if (!overallStatus.includes("successfully")) { // Keep button disabled only if setup didn't succeed
          setIsSettingUp(false);
      }
      listeners.forEach(unlisten => unlisten());
      console.log("[SetupPage] Event listeners cleaned up.");
    }
  // Remove onSetupComplete from dependency array
  }, [router, overallStatus]);

  const progressPercent = vlmTotalSize > 0 ? (vlmCurrentProgress / vlmTotalSize) * 100 : 0;

  // Render the setup card directly. The parent (RootLayout) handles centering/layout.
  return (
    <div className="flex items-center justify-center w-screen h-screen bg-background"> {/* Use theme background */}
       <Toaster richColors position="top-center" />
      <Card className="w-[450px] shadow-lg">
        <CardHeader>
          <CardTitle className="text-2xl">Application Setup Required</CardTitle>
          <CardDescription>
            Essential models need to be downloaded before using the application.
            This might take some time depending on your internet connection. The total download size is approximately 0.7 GB.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {overallStatus !== "" ? 
          (<div className="text-sm text-muted-foreground"> {/* Use theme text color */}
            {overallStatus}
          </div>) : (
            <div className="h-[20px]" />
          )}

          {/* VLM Progress Display */}
          {isSettingUp && vlmDownloadId !== null && (
            <div className="space-y-2 pt-2">
              <div className="flex justify-between text-xs font-medium text-foreground"> {/* Use theme text color */}
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
          {/* Button logic remains similar, but no isSetupComplete check needed here */}
          {!isSettingUp && (
            <Button onClick={handleStartSetup} className="w-full">
              Start Setup
            </Button>
          )}
           {isSettingUp && (
             <Button className="w-full" disabled={true}>
               Setting Up...
             </Button>
           )}
        </CardFooter>
      </Card>
    </div>
  );
}