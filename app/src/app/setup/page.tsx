"use client"; // Required for hooks like useState, useEffect

import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { redirect } from 'next/navigation'
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


function SetupPage() {
  const [isLoading, setIsLoading] = useState(true);
  const [isSetupComplete, setIsSetupComplete] = useState(false);
  const [isSettingUp, setIsSettingUp] = useState(false);
  // setupError state might not be needed if only using toasts for errors
  // const [setupError, setSetupError] = useState<string | null>(null);
  const [overallStatus, setOverallStatus] = useState("Checking setup status...");

  // VLM Download specific state
  const [vlmDownloadId, setVlmDownloadId] = useState<number | null>(null);
  const [vlmTotalSize, setVlmTotalSize] = useState(0);
  const [vlmCurrentProgress, setVlmCurrentProgress] = useState(0);

  // Check setup status on component mount
  useEffect(() => {
    const checkStatus = async () => {
      setIsLoading(true);
      // setSetupError(null); // Clear previous errors if state is kept
      setOverallStatus("Checking setup status...");
      try {
        const complete = await invoke<boolean>("check_setup_complete");
        if (complete) {
          setIsSetupComplete(true);
          setOverallStatus("Setup is complete. Redirecting...");
          console.log("[SetupPage] Setup complete, navigating to /");
          toast.success("Setup already complete!"); // Optional success toast
          redirect("/")
        } else {
          setIsSetupComplete(false);
          setOverallStatus("Models need to be downloaded.");
          console.log("[SetupPage] Setup required.");
        }
      } catch (err) {
        const errorMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'An unknown error occurred');
        console.error("[SetupPage] Failed to check setup status:", err);
        // Use toast for error instead of Alert
        toast.error(`Failed to check setup status: ${errorMsg}`);
        setOverallStatus("Error checking setup status.");
      } finally {
        setIsLoading(false);
      }
    };
    checkStatus();
  }, []);

  // Function to start the setup process
  const handleStartSetup = useCallback(async () => {
    setIsSettingUp(true);
    // setSetupError(null); // Clear previous errors if state is kept
    setOverallStatus("Starting setup...");
    setVlmDownloadId(null);
    setVlmTotalSize(0);
    setVlmCurrentProgress(0);

    const listeners: UnlistenFn[] = [];

    try {
      // Setup listeners (remain the same)
      listeners.push(await listen<DownloadStartedPayload>('download-started', (event) => {
        console.log("Download Started:", event.payload);
        setVlmDownloadId(event.payload.id);
        setVlmTotalSize(event.payload.contentLength);
        setVlmCurrentProgress(0);
        setOverallStatus(`Downloading VLM model ${event.payload.id}...`);
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
            setOverallStatus("VLM models downloaded. Initializing embedding model (this may take a moment)...");
            setVlmDownloadId(null);
            setVlmTotalSize(0);
            setVlmCurrentProgress(0);
        } else if (event.payload.id === 1) {
             setOverallStatus("VLM model 1 finished. Starting model 2...");
        }
      }));

      // Invoke the setup command
      const result = await invoke<string>("setup");
      console.log("[SetupPage] Setup command finished:", result);
      setOverallStatus("Setup completed successfully! Redirecting...");
      setIsSetupComplete(true);
      toast.success("Setup completed successfully!"); // Success toast
      setTimeout(() => redirect("/"), 1500);

    } catch (err) {
      console.error("[SetupPage] Setup failed:", err);
      const errorMsg = typeof err === 'string' ? err : (err instanceof Error ? err.message : 'An unknown error occurred');
      // Use toast for error instead of Alert
      toast.error(`Setup failed: ${errorMsg}`);
      setOverallStatus("Setup process encountered an error.");
    } finally {
      setIsSettingUp(false);
      listeners.forEach(unlisten => unlisten());
      console.log("[SetupPage] Event listeners cleaned up.");
    }
  }, []);

  // Render logic (remains mostly the same, Alert removed)
  if (isLoading) {
    return (
      <div className="flex items-center justify-center">
        {/* Add Toaster here so it's available during loading */}
        <Toaster richColors position="top-center" />
        <Card className="w-[100px]">
          <CardHeader>
            <CardTitle>Setup</CardTitle>
          </CardHeader>
          <CardContent>
            <p>{overallStatus}</p>
          </CardContent>
        </Card>
      </div>
    );
  }

  if (isSetupComplete && !isSettingUp) {
     return (
        <div className="flex items-center justify-center w-screen h-screen">
            {/* Add Toaster here too */}
            <Toaster richColors position="top-center" />
            <p>Setup complete. Redirecting...</p>
        </div>
     );
  }

  const progressPercent = vlmTotalSize > 0 ? (vlmCurrentProgress / vlmTotalSize) * 100 : 0;

  return (
    <div className="flex items-center justify-center w-screen h-screen bg-gray-100 dark:bg-gray-900">
       {/* Add Toaster here for the main setup view */}
       <Toaster richColors position="top-center" />
      <Card className="w-[450px] shadow-lg">
        <CardHeader>
          <CardTitle className="text-2xl">Model Setup Required</CardTitle>
          <CardDescription>
            Essential models need to be downloaded before using the application.
            This might take some time depending on your internet connection. The total download size is approximately 1.5 GB.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Removed the Alert component */}
          {/* {setupError && ( ... )} */}

          <div className="text-sm text-gray-600 dark:text-gray-400">
            Status: {overallStatus}
          </div>

          {/* VLM Progress Display (remains the same) */}
          {isSettingUp && vlmDownloadId !== null && (
            <div className="space-y-2 pt-2">
              <div className="flex justify-between text-xs font-medium text-gray-700 dark:text-gray-300">
                <span>
                  {`VLM Model ${vlmDownloadId}`} ({formatBytes(vlmCurrentProgress)} / {formatBytes(vlmTotalSize)})
                </span>
                <span>{progressPercent.toFixed(0)}%</span>
              </div>
              <Progress value={progressPercent} className="w-full h-2" />
            </div>
          )}
          {/* Embedding Model Status Display (remains the same) */}
           {isSettingUp && vlmDownloadId === null && overallStatus.includes("embedding model") && (
             <div className="space-y-2 pt-2">
                <p className="text-xs text-center text-gray-500">Processing embedding model (console may show progress)...</p>
                <Progress value={100} className="w-full h-2 opacity-50 animate-pulse" />
             </div>
           )}

        </CardContent>
        <CardFooter>
          {/* Buttons remain the same */}
          {!isSettingUp && !isSetupComplete && (
            <Button onClick={handleStartSetup} className="w-full" disabled={isLoading}>
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

export default SetupPage;