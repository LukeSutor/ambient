"use client";

import Link from "next/link";
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Chrome } from "lucide-react";

export default function Integrations() {
  return (
    <div className="relative flex flex-col items-center justify-center p-4 w-full">
      <div className="w-full max-w-xl">
        <Card className="w-full">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Chrome className="w-5 h-5 text-blue-500" /> Chromium Integration
            </CardTitle>
            <CardDescription>
              Record and replay browser workflows using the Chromium extension and integration server.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <div className="text-sm text-muted-foreground">
              View, manage, and run your saved browser workflows. Requires the browser extension and the integration server to be running.
            </div>
          </CardContent>
          <CardFooter>
            <Button asChild variant="default">
              <Link href="/integrations/chromium">Open Chromium Workflows</Link>
            </Button>
          </CardFooter>
        </Card>
      </div>
    </div>
  );
}