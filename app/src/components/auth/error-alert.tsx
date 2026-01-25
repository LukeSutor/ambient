"use client";

import { AlertCircle } from "lucide-react";

interface ErrorAlertProps {
  error: string | null;
}

export function ErrorAlert({ error }: ErrorAlertProps) {
  if (!error) return null;

  return (
    <div className="flex items-center space-x-2 text-red-600 bg-red-50 p-3 rounded-md border border-red-200">
      <AlertCircle className="h-4 w-4 flex-shrink-0" />
      <span className="text-sm">{error}</span>
    </div>
  );
}
