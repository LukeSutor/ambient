"use client";

import Link from "next/link";
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Chrome } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

export default function Integrations() {
  const [chromiumStatus, setChromiumStatus] = useState<'connected' | 'disconnected' | 'error' | 'checking'>('checking');
  const [chromiumError, setChromiumError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    setChromiumStatus('checking');
    setChromiumError(null);
    invoke("ping_chromium_extension")
      .then(() => {
        if (!cancelled) setChromiumStatus('connected');
      })
      .catch((err: any) => {
        if (!cancelled) {
          setChromiumStatus('disconnected');
          setChromiumError(typeof err === 'string' ? err : (err?.message || 'Unknown error'));
        }
      });
    return () => { cancelled = true; };
  }, []);

  return (
    <div className="relative flex flex-col items-center justify-center p-4 w-full">
      <div className="w-full max-w-xl">
        <Card className="w-full">
          <CardHeader>
            <div className="flex justify-between items-center gap-2">
              <CardTitle className="flex items-center gap-2">
                <Chrome className="w-5 h-5 text-blue-500" /> Chromium Integration
              </CardTitle>
              <Badge
                variant={chromiumStatus === 'connected' ? 'default' : chromiumStatus === 'checking' ? 'secondary' : 'destructive'}
                className={chromiumStatus === 'connected' ? 'bg-green-100 text-green-800 font-bold' : ''}
              >
                {chromiumStatus === 'checking' && 'Checking...'}
                {chromiumStatus === 'connected' && 'Connected'}
                {chromiumStatus === 'disconnected' && 'Disconnected'}
                {chromiumStatus === 'error' && 'Error'}
              </Badge>
            </div>
            <CardDescription>
              Record and replay browser workflows using the Chromium extension and integration server.
              {chromiumError && (
                <div className="text-xs text-red-500 mt-1">{chromiumError}</div>
              )}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground">
              View, manage, and run your saved browser workflows. Requires the browser extension and the integration server to be running.
            </div>
          </CardContent>
          <CardFooter>
            <Button asChild variant="secondary">
              <Link href="/integrations/chromium">Open Workflows</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}