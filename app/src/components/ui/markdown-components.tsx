import { cn } from "@/lib/utils";
import React from "react";
import { type Components, defaultUrlTransform } from "react-markdown";

// Custom URL transformer for enhanced security and functionality
export const customUrlTransform = (
  url: string,
  _key: string,
  _node: unknown,
) => {
  // Use default security but add custom logic for internal links
  const safeUrl = defaultUrlTransform(url);

  // Handle relative URLs for your app
  if (url.startsWith("/")) {
    return url; // Keep internal app links as-is
  }

  // Add utm parameters for external links if needed
  if (safeUrl.startsWith("http") && !url.includes("utm_source")) {
    try {
      const urlObj = new URL(safeUrl);
      urlObj.searchParams.set("utm_source", "ambient");
      return urlObj.toString();
    } catch {
      return safeUrl;
    }
  }

  return safeUrl;
};

export const markdownComponents: Components = {
  // Headings
  h1: ({ className, ...props }) => (
    <h1
      className={cn(
        "mt-6 mb-4 text-2xl font-bold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h2: ({ className, ...props }) => (
    <h2
      className={cn(
        "mt-6 mb-4 text-xl font-semibold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h3: ({ className, ...props }) => (
    <h3
      className={cn(
        "mt-5 mb-3 text-lg font-semibold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h4: ({ className, ...props }) => (
    <h4
      className={cn(
        "mt-4 mb-2 text-base font-semibold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h5: ({ className, ...props }) => (
    <h5
      className={cn(
        "mt-3 mb-2 text-sm font-semibold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h6: ({ className, ...props }) => (
    <h6
      className={cn(
        "mt-3 mb-2 text-xs font-semibold tracking-tight text-gray-900 first:mt-0",
        className,
      )}
      {...props}
    />
  ),

  // Paragraphs and text styling
  p: ({ className, ...props }) => (
    <p
      className={cn(
        "leading-7 text-gray-800 [&:not(:first-child)]:mt-4",
        className,
      )}
      {...props}
    />
  ),

  // Links
  a: ({ className, ...props }) => (
    <a
      className={cn(
        "font-medium text-blue-600 hover:text-blue-800 underline underline-offset-4 transition-colors",
        className,
      )}
      target="_blank"
      rel="noopener noreferrer"
      {...props}
    />
  ),

  // Text formatting
  strong: ({ className, ...props }) => (
    <strong
      className={cn("font-semibold text-gray-900", className)}
      {...props}
    />
  ),

  em: ({ className, ...props }) => (
    <em className={cn("italic", className)} {...props} />
  ),

  del: ({ className, ...props }) => (
    <del className={cn("line-through opacity-70", className)} {...props} />
  ),

  // Line break and horizontal rule
  br: ({ className, ...props }) => (
    <br className={cn("", className)} {...props} />
  ),

  hr: ({ className, ...props }) => (
    <hr className={cn("my-6 border-t border-gray-300", className)} {...props} />
  ),

  // Lists
  ul: ({ className, ...props }) => (
    <ul
      className={cn("my-4 ml-6 list-disc space-y-2 [&>li]:mt-1", className)}
      {...props}
    />
  ),

  ol: ({ className, ...props }) => (
    <ol
      className={cn("my-4 ml-6 list-decimal space-y-2 [&>li]:mt-1", className)}
      {...props}
    />
  ),

  li: ({ className, ...props }) => (
    <li className={cn("text-gray-800", className)} {...props} />
  ),

  // Blockquotes
  blockquote: ({ className, ...props }) => (
    <blockquote
      className={cn(
        "mt-4 mb-4 border-l-4 border-gray-300 bg-gray-50/40 pl-4 py-2 italic text-gray-700",
        className,
      )}
      {...props}
    />
  ),

  // Code with enhanced language detection
  code: ({ className, children, ...props }) => {
    const match = /language-(\w+)/.exec(className || "");
    const language = match ? match[1] : "";
    const isInline =
      !className?.includes("language-") &&
      typeof children === "string" &&
      !children.includes("\n");

    if (isInline) {
      return (
        <code
          className={cn(
            "relative rounded bg-white/20 px-[0.2rem] font-mono text-sm font-medium text-black border border-white/30",
            className,
          )}
          {...props}
        >
          {children}
        </code>
      );
    }

    // Block code with language indicator
    return (
      <div className="flex flex-col">
        <div className="w-min whitespace-nowrap text-xs text-black/80 rounded font-mono mb-1">
          {language || "text"}
        </div>
        <code
          className={cn("block text-sm font-mono text-black/80", className)}
          {...props}
        >
          {children}
        </code>
      </div>
    );
  },

  pre: ({ className, children, ...props }) => (
    <pre
      className={cn(
        "mt-4 mb-4 overflow-x-auto rounded-lg bg-white/20 p-2 border border-white/30 relative",
        className,
      )}
      {...props}
    >
      {children}
    </pre>
  ),

  // Images
  img: ({ className, alt, ...props }) => (
    /* biome-ignore lint/a11y/useAltText: Alt text provided via prop or default */
    <img
      className={cn("mt-4 mb-4 max-w-full rounded-lg shadow-sm", className)}
      alt={alt || "Conversation content"}
      {...props}
    />
  ),

  // Tables (from remark-gfm)
  table: ({ className, ...props }) => (
    <div className="my-6 w-full overflow-y-auto">
      <table
        className={cn(
          "w-full border-collapse border border-gray-300 text-sm",
          className,
        )}
        {...props}
      />
    </div>
  ),

  thead: ({ className, ...props }) => (
    <thead className={cn("bg-gray-50/40", className)} {...props} />
  ),

  tbody: ({ className, ...props }) => (
    <tbody className={cn("[&_tr:last-child]:border-0", className)} {...props} />
  ),

  tr: ({ className, ...props }) => (
    <tr
      className={cn(
        "border-b border-gray-300 transition-colors hover:bg-gray-50/50",
        className,
      )}
      {...props}
    />
  ),

  th: ({ className, ...props }) => (
    <th
      className={cn(
        "border border-gray-300 px-4 py-2 text-left font-semibold text-gray-900 [&[align=center]]:text-center [&[align=right]]:text-right",
        className,
      )}
      {...props}
    />
  ),

  td: ({ className, ...props }) => (
    <td
      className={cn(
        "border border-gray-300 px-4 py-2 text-gray-800 [&[align=center]]:text-center [&[align=right]]:text-right",
        className,
      )}
      {...props}
    />
  ),

  // Task list items (from remark-gfm)
  input: ({ className, ...props }) => (
    <input
      className={cn(
        "mr-2 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50",
        className,
      )}
      disabled
      {...props}
    />
  ),
};

// Alternative dark theme components (optional)
export const darkMarkdownComponents: Components = {
  ...markdownComponents,

  // Override specific components for dark theme
  h1: ({ className, ...props }) => (
    <h1
      className={cn(
        "mt-6 mb-4 text-2xl font-bold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h2: ({ className, ...props }) => (
    <h2
      className={cn(
        "mt-6 mb-4 text-xl font-semibold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h3: ({ className, ...props }) => (
    <h3
      className={cn(
        "mt-5 mb-3 text-lg font-semibold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h4: ({ className, ...props }) => (
    <h4
      className={cn(
        "mt-4 mb-2 text-base font-semibold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h5: ({ className, ...props }) => (
    <h5
      className={cn(
        "mt-3 mb-2 text-sm font-semibold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),
  h6: ({ className, ...props }) => (
    <h6
      className={cn(
        "mt-3 mb-2 text-xs font-semibold tracking-tight text-white first:mt-0",
        className,
      )}
      {...props}
    />
  ),

  p: ({ className, ...props }) => (
    <p
      className={cn(
        "leading-7 text-gray-200 [&:not(:first-child)]:mt-4",
        className,
      )}
      {...props}
    />
  ),

  strong: ({ className, ...props }) => (
    <strong className={cn("font-semibold text-white", className)} {...props} />
  ),

  li: ({ className, ...props }) => (
    <li className={cn("text-gray-200", className)} {...props} />
  ),

  blockquote: ({ className, ...props }) => (
    <blockquote
      className={cn(
        "mt-4 mb-4 border-l-4 border-gray-600 bg-gray-800 pl-4 py-2 italic text-gray-300",
        className,
      )}
      {...props}
    />
  ),

  code: ({ className, children, ...props }) => {
    const match = /language-(\w+)/.exec(className || "");
    const language = match ? match[1] : "";
    const isInline =
      !className?.includes("language-") &&
      typeof children === "string" &&
      !children.includes("\n");

    if (isInline) {
      return (
        <code
          className={cn(
            "relative rounded bg-gray-800/20 px-[0.3rem] py-[0.2rem] font-mono text-sm font-medium text-gray-100 border border-gray-600/30",
            className,
          )}
          {...props}
        >
          {children}
        </code>
      );
    }

    return (
      <div className="relative">
        {language && (
          <div className="absolute top-2 right-2 text-xs text-gray-400 bg-gray-700/80 px-2 py-1 rounded uppercase font-mono">
            {language}
          </div>
        )}
        <code
          className={cn("block text-sm font-mono text-gray-100 p-0", className)}
          {...props}
        >
          {children}
        </code>
      </div>
    );
  },

  pre: ({ className, children, ...props }) => (
    <pre
      className={cn(
        "mt-4 mb-4 overflow-x-auto rounded-lg bg-gray-900 p-4 text-gray-100 border border-gray-700/50 relative",
        className,
      )}
      {...props}
    >
      {children}
    </pre>
  ),

  td: ({ className, ...props }) => (
    <td
      className={cn(
        "border border-gray-600 px-4 py-2 text-gray-200 [&[align=center]]:text-center [&[align=right]]:text-right",
        className,
      )}
      {...props}
    />
  ),

  th: ({ className, ...props }) => (
    <th
      className={cn(
        "border border-gray-600 px-4 py-2 text-left font-semibold text-white [&[align=center]]:text-center [&[align=right]]:text-right",
        className,
      )}
      {...props}
    />
  ),
};
