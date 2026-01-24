"use client";

import { SiteHeader } from "@/components/site-header";
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
import { Toaster } from "@/components/ui/sonner";
import { useSetup } from "@/lib/setup/useSetup";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { X } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";

export default function SetupPage() {
  const router = useRouter();

  // Setup state
  const {
    isDownloading,
    downloadingId,
    formattedDownloadedBytes,
    formattedTotalContentLength,
    totalDownloadedBytes,
    totalContentLength,
    setupMessage,
    startSetup,
  } = useSetup();

  // Function to start the setup process
  const handleStartSetup = useCallback(async () => {
    console.log("[SetupPage] Starting setup process...");
    try {
      await startSetup();
      toast.success("Setup completed successfully!");

      // Redirect to dashboard after successful setup
      router.push("/secondary");
    } catch (err) {
      console.error("[SetupPage] Setup failed:", err);
      const errorMsg =
        typeof err === "string"
          ? err
          : err instanceof Error
            ? err.message
            : "An unknown error occurred";
      toast.error(`Setup failed: ${errorMsg}`);
    }
  }, [router, startSetup]);

  // Reroute if no download needed, this fixes the issue of the page sometimes freezing on setup
  // a better solution is probably needed
  useEffect(() => {
    if (totalContentLength === 0) {
      router.push("/secondary");
    }
  }, [router, totalContentLength]);

  const progressPercent =
    totalContentLength > 0
      ? (totalDownloadedBytes / totalContentLength) * 100
      : 0;

  return (
    <div className="flex items-center justify-center w-screen h-screen bg-background p-4">
      <SiteHeader includeMinimize />
      <Toaster richColors position="top-center" />

      {/* Setup Card */}
      <Card className="relative w-full max-w-[450px] pt-12 shadow-lg">

        <CardHeader className="text-center pt-2">
          <CardTitle className="text-2xl font-bold">
            Application Setup Required
          </CardTitle>
          <CardDescription>
            Essential files need to be downloaded before using the application.
            This might take some time depending on your internet connection. The
            total download size is
            {totalContentLength > 0 ? (
              ` ${formattedTotalContentLength}.`
            ) : (
              <div className="inline-block h-[16px] -mb-[2px] w-12 bg-muted rounded mx-1 animate-pulse" />
            )}
          </CardDescription>
        </CardHeader>

        <CardContent className="space-y-4">
          {setupMessage !== "" && (
            <div className="text-sm text-muted-foreground">{setupMessage}</div>
          )}

          {/* Progress Display */}
          {isDownloading && downloadingId !== null && (
            <div className="space-y-2 pt-2">
              <div className="flex justify-between text-xs font-medium text-foreground">
                <span>
                  {`Model ${downloadingId}`} ({formattedDownloadedBytes} /{" "}
                  {formattedTotalContentLength})
                </span>
                <span>{progressPercent.toFixed(0)}%</span>
              </div>
              <Progress value={progressPercent} className="w-full h-2" />
            </div>
          )}
        </CardContent>

        <CardFooter>
          {!isDownloading && (
            <Button
              onClick={() => {
                void handleStartSetup();
              }}
              className="w-full h-11 text-base font-medium"
            >
              Start Setup
            </Button>
          )}
          {isDownloading && (
            <Button
              className="w-full h-11 text-base font-medium"
              disabled={true}
            >
              Setting Up...
            </Button>
          )}
        </CardFooter>
      </Card>
    </div>
  );
}
