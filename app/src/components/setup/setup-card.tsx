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
  isDownloading: boolean;
  downloadingId: number | null;
  formattedDownloadedBytes: string;
  formattedTotalContentLength: string;
  totalDownloadedBytes: number;
  totalContentLength: number;
  onStartSetup: () => void;
  className?: string;
}

export function SetupCard({
  isDownloading,
  downloadingId,
  formattedDownloadedBytes,
  formattedTotalContentLength,
  totalDownloadedBytes,
  totalContentLength,
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
        <CardTitle className="text-2xl font-bold">Setup Required</CardTitle>
        <CardDescription>
          Essential files need to be downloaded before using ambient. This might
          take some time depending on your internet connection. The total
          download size is
          {totalContentLength > 0 ? (
            ` ${formattedTotalContentLength}.`
          ) : (
            <span className="inline-block h-[16px] -mb-[2px] w-12 bg-muted rounded mx-1 animate-pulse" />
          )}
        </CardDescription>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* Progress Display */}
        {isDownloading && downloadingId !== null && (
          <div className="space-y-2 pt-2">
            <div className="flex justify-between text-xs font-medium text-foreground tabular-nums font-mono">
              <div className="flex items-center justify-between w-[18ch]">
                <span>{formattedDownloadedBytes}</span>
                <span className="text-muted-foreground">/</span>
                <span>{formattedTotalContentLength}</span>
              </div>
              <span className="w-[8ch] text-right">
                {progressPercent.toFixed(0)}%
              </span>
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
          <Button className="w-full h-11 text-base font-medium" disabled={true}>
            Setting Up...
          </Button>
        )}
      </CardFooter>
    </Card>
  );
}
