"use client";

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
import { cn } from "@/lib/utils";

export interface SetupCardProps {
  /** Whether a download is currently in progress */
  isDownloading: boolean;
  /** The ID of the currently downloading item */
  downloadingId: number | null;
  /** Formatted string of downloaded bytes (e.g., "50 MB") */
  formattedDownloadedBytes: string;
  /** Formatted string of total content length (e.g., "100 MB") */
  formattedTotalContentLength: string;
  /** Total bytes downloaded so far */
  totalDownloadedBytes: number;
  /** Total content length in bytes */
  totalContentLength: number;
  /** Current setup status message */
  setupMessage: string;
  /** Callback when the start setup button is clicked */
  onStartSetup: () => void;
  /** Additional class names for the Card component */
  className?: string;
}

export function SetupCard({
  isDownloading,
  downloadingId,
  formattedDownloadedBytes,
  formattedTotalContentLength,
  totalDownloadedBytes,
  totalContentLength,
  setupMessage,
  onStartSetup,
  className,
}: SetupCardProps) {
  const progressPercent =
    totalContentLength > 0
      ? (totalDownloadedBytes / totalContentLength) * 100
      : 0;

  return (
    <Card className={cn("relative w-full pt-12", className)}>
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
            <span className="inline-block h-[16px] -mb-[2px] w-12 bg-muted rounded mx-1 animate-pulse" />
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
            onClick={onStartSetup}
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
  );
}
