import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: false,
  devIndicators: false,
  output: "export",
  images: {
    unoptimized: true,
  },
  distDir: "dist",
  typescript: {
    ignoreBuildErrors: true, 
  },
  eslint: {
    ignoreDuringBuilds: true,
  },
  serverExternalPackages: ["@tauri-apps/api"],
  experimental: {
    cpus: 1,
    workerThreads: false,
  },
};

export default nextConfig;
