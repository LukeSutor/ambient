import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  output: "export",
  images: {
    unoptimized: true,
  },
  distDir: "dist",
  experimental: {
    // Help prevent hydration issues with Tauri components
    serverComponentsExternalPackages: ["@tauri-apps/api"],
  },
};

export default nextConfig;
