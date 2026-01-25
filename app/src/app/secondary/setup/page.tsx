"use client";

import { SetupCard } from "@/components/setup";
import { useRoleAccess } from "@/lib/role-access/useRoleAccess";
import { useSetup } from "@/lib/setup/useSetup";
import { useRouter } from "next/navigation";
import { useCallback, useEffect } from "react";
import { toast } from "sonner";

export default function SetupPage() {
  const router = useRouter();
  const { isSetupComplete } = useRoleAccess();

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

  const handleStartSetup = useCallback(async () => {
    console.log("[SetupPage] Starting setup process...");
    try {
      await startSetup();
      toast.success("Setup completed successfully!");
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

  useEffect(() => {
    if (isSetupComplete) {
      router.push("/secondary");
    }
  }, [router, isSetupComplete]);

  return (
    <div className="flex items-center justify-center w-full h-full bg-background p-4">
      <SetupCard
        isDownloading={isDownloading}
        downloadingId={downloadingId}
        formattedDownloadedBytes={formattedDownloadedBytes}
        formattedTotalContentLength={formattedTotalContentLength}
        totalDownloadedBytes={totalDownloadedBytes}
        totalContentLength={totalContentLength}
        setupMessage={setupMessage}
        onStartSetup={() => void handleStartSetup()}
        className="max-w-[450px] shadow-lg"
      />
    </div>
  );
}
