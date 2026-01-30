"use client";

import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

type ScrapeLoadPayload = {
  requestId: string;
  url: string;
};

type ScrapeSetHtmlPayload = {
  requestId: string;
  html: string;
};

type ScrapeErrorPayload = {
  requestId: string;
  error: string;
};

export default function WebviewScraperPage() {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const currentRequestIdRef = useRef<string | null>(null);
  const [currentUrl, setCurrentUrl] = useState<string | null>(null);
  const [status, setStatus] = useState("Idle");

  useEffect(() => {
    console.log({ iframeRef: iframeRef.current });
    let unlistenLoad: UnlistenFn | undefined;
    let unlistenHtml: UnlistenFn | undefined;
    let unlistenError: UnlistenFn | undefined;

    const setup = async () => {
      unlistenLoad = await listen<ScrapeLoadPayload>(
        "webview_scraper_load",
        (event) => {
          console.log("Received load event:", event);
          const { requestId, url } = event.payload;
          currentRequestIdRef.current = requestId;
          setCurrentUrl(url);
          setStatus("Loading");

          const iframe = iframeRef.current;
          if (iframe) {
            iframe.removeAttribute("srcdoc");
            iframe.src = url;
          }
        },
      );

      unlistenHtml = await listen<ScrapeSetHtmlPayload>(
        "webview_scraper_set_html",
        (event) => {
          console.log("Received set_html event:", event);
          const { requestId, html } = event.payload;
          if (currentRequestIdRef.current && requestId !== currentRequestIdRef.current) {
            return;
          }
          const iframe = iframeRef.current;
          if (iframe) {
            iframe.src = "about:blank";
            iframe.srcdoc = html;
          }
          setStatus("HTML ready");
        },
      );

      unlistenError = await listen<ScrapeErrorPayload>(
        "webview_scraper_error",
        (event) => {
          console.log("Received error event:", event);
          const { requestId, error } = event.payload;
          if (currentRequestIdRef.current && requestId !== currentRequestIdRef.current) {
            return;
          }
          setStatus(`Error: ${error}`);
        },
      );
    };

    void setup();

    (window as { __scraper?: unknown }).__scraper = {
      getIframeDocument: () => iframeRef.current?.contentDocument ?? null,
      getIframeWindow: () => iframeRef.current?.contentWindow ?? null,
    };

    return () => {
      unlistenLoad?.();
      unlistenHtml?.();
      unlistenError?.();
      delete (window as { __scraper?: unknown }).__scraper;
    };
  }, [iframeRef.current]);

  return (
    <div className="flex h-screen w-screen flex-col bg-black text-white">
      <div className="flex h-10 items-center justify-between gap-3 px-3 text-xs">
        <span className="font-medium">Webview Scraper</span>
        <span className="truncate text-white/70">{currentUrl ?? "No URL"}</span>
        <span className="text-white/70">{status}</span>
      </div>
      <iframe
        ref={iframeRef}
        title="Webview Scraper"
        className="h-[calc(100vh-2.5rem)] w-full border-0"
        sandbox="allow-same-origin allow-scripts allow-forms allow-popups"
      />
    </div>
  );
}
