"use client";

import { SetupCard } from "@/components/setup";
import { useRoleAccess } from "@/lib/role-access";
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
      router.push("/hud");
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
      router.push("/hud");
    }
  }, [router, isSetupComplete]);

  return (
    <SetupCard
      isDownloading={isDownloading}
      downloadingId={downloadingId}
      formattedDownloadedBytes={formattedDownloadedBytes}
      formattedTotalContentLength={formattedTotalContentLength}
      totalDownloadedBytes={totalDownloadedBytes}
      totalContentLength={totalContentLength}
      setupMessage={setupMessage}
      onStartSetup={() => void handleStartSetup()}
      className="rounded-none border-none shadow-none"
    />
  );
}
