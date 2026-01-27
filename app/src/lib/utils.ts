import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

export function formatBytes(bytes: number, decimals = 1): string {
  if (bytes === 0) return "0 Bytes";

  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ["Bytes", "KB", "MB", "GB"];

  let i = Math.floor(Math.log(bytes) / Math.log(k));

  // Shift to the next unit if we reach 1000 (4 digits) to avoid jumping from 3 to 4 digits
  if (i < sizes.length - 1 && bytes / k ** i >= 1000) {
    i++;
  }

  const value = bytes / k ** i;

  // Use 2 decimals if value is < 1 (from unit shift) or for small GB values (< 10GB)
  // to provide enough detail while maintaining a stable width.
  const precision = i > 0 && (value < 1 || (i === 3 && value < 10)) ? 2 : dm;

  return `${Number.parseFloat(value.toFixed(precision))} ${sizes[i]}`;
}
