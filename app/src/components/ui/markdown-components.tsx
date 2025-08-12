import React from 'react';
import { Components } from 'react-markdown';
import { cn } from '@/lib/utils';

export const markdownComponents: Components = {
  // Headings
  h1: ({ className, ...props }) => (
    <h1
      className={cn(
        "mt-6 mb-4 text-2xl font-bold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h2: ({ className, ...props }) => (
    <h2
      className={cn(
        "mt-6 mb-4 text-xl font-semibold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h3: ({ className, ...props }) => (
    <h3
      className={cn(
        "mt-5 mb-3 text-lg font-semibold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h4: ({ className, ...props }) => (
    <h4
      className={cn(
        "mt-4 mb-2 text-base font-semibold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h5: ({ className, ...props }) => (
    <h5
      className={cn(
        "mt-3 mb-2 text-sm font-semibold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h6: ({ className, ...props }) => (
    <h6
      className={cn(
        "mt-3 mb-2 text-xs font-semibold tracking-tight text-gray-900 first:mt-0",
        className
      )}
      {...props}
    />
  ),
  
  // Paragraphs and text styling
  p: ({ className, ...props }) => (
    <p
      className={cn(
        "leading-7 text-gray-800 [&:not(:first-child)]:mt-4",
        className
      )}
      {...props}
    />
  ),
  
  // Links
  a: ({ className, ...props }) => (
    <a
      className={cn(
        "font-medium text-blue-600 hover:text-blue-800 underline underline-offset-4 transition-colors",
        className
      )}
      target="_blank"
      rel="noopener noreferrer"
      {...props}
    />
  ),
  
  // Text formatting
  strong: ({ className, ...props }) => (
    <strong className={cn("font-semibold text-gray-900", className)} {...props} />
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
      className={cn(
        "my-4 ml-6 list-disc space-y-2 [&>li]:mt-1",
        className
      )}
      {...props}
    />
  ),
  
  ol: ({ className, ...props }) => (
    <ol
      className={cn(
        "my-4 ml-6 list-decimal space-y-2 [&>li]:mt-1",
        className
      )}
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
        className
      )}
      {...props}
    />
  ),
  
  // Code
  code: ({ className, ...props }: any) => {
    const inline = props.inline;
    if (inline) {
      return (
        <code
          className={cn(
            "relative rounded bg-gray-100/20 px-[0.3rem] py-[0.2rem] font-mono text-sm font-medium text-gray-900",
            className
          )}
          {...props}
        />
      );
    }
    
    return (
      <code
        className={cn(
          "relative rounded font-mono text-sm font-medium",
          className
        )}
        {...props}
      />
    );
  },
  
  pre: ({ className, ...props }) => (
    <pre
      className={cn(
        "mt-4 mb-4 overflow-x-auto rounded-lg bg-gray-900 p-4",
        className
      )}
      {...props}
    />
  ),
  
  // Images
  img: ({ className, alt, ...props }) => (
    <img
      className={cn(
        "mt-4 mb-4 max-w-full rounded-lg shadow-sm",
        className
      )}
      alt={alt}
      {...props}
    />
  ),
  
  // Tables (from remark-gfm)
  table: ({ className, ...props }) => (
    <div className="my-6 w-full overflow-y-auto">
      <table
        className={cn(
          "w-full border-collapse border border-gray-300 text-sm",
          className
        )}
        {...props}
      />
    </div>
  ),
  
  thead: ({ className, ...props }) => (
    <thead
      className={cn(
        "bg-gray-50/40",
        className
      )}
      {...props}
    />
  ),
  
  tbody: ({ className, ...props }) => (
    <tbody
      className={cn(
        "[&_tr:last-child]:border-0",
        className
      )}
      {...props}
    />
  ),
  
  tr: ({ className, ...props }) => (
    <tr
      className={cn(
        "border-b border-gray-300 transition-colors hover:bg-gray-50/50",
        className
      )}
      {...props}
    />
  ),
  
  th: ({ className, ...props }) => (
    <th
      className={cn(
        "border border-gray-300 px-4 py-2 text-left font-semibold text-gray-900 [&[align=center]]:text-center [&[align=right]]:text-right",
        className
      )}
      {...props}
    />
  ),
  
  td: ({ className, ...props }) => (
    <td
      className={cn(
        "border border-gray-300 px-4 py-2 text-gray-800 [&[align=center]]:text-center [&[align=right]]:text-right",
        className
      )}
      {...props}
    />
  ),
  
  // Task list items (from remark-gfm)
  input: ({ className, ...props }) => (
    <input
      className={cn(
        "mr-2 h-4 w-4 rounded border-gray-300 text-blue-600 focus:ring-blue-500 disabled:cursor-not-allowed disabled:opacity-50",
        className
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
        className
      )}
      {...props}
    />
  ),
  h2: ({ className, ...props }) => (
    <h2
      className={cn(
        "mt-6 mb-4 text-xl font-semibold tracking-tight text-white first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h3: ({ className, ...props }) => (
    <h3
      className={cn(
        "mt-5 mb-3 text-lg font-semibold tracking-tight text-white first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h4: ({ className, ...props }) => (
    <h4
      className={cn(
        "mt-4 mb-2 text-base font-semibold tracking-tight text-white first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h5: ({ className, ...props }) => (
    <h5
      className={cn(
        "mt-3 mb-2 text-sm font-semibold tracking-tight text-white first:mt-0",
        className
      )}
      {...props}
    />
  ),
  h6: ({ className, ...props }) => (
    <h6
      className={cn(
        "mt-3 mb-2 text-xs font-semibold tracking-tight text-white first:mt-0",
        className
      )}
      {...props}
    />
  ),
  
  p: ({ className, ...props }) => (
    <p
      className={cn(
        "leading-7 text-gray-200 [&:not(:first-child)]:mt-4",
        className
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
        className
      )}
      {...props}
    />
  ),
  
  code: ({ className, ...props }: any) => {
    const inline = props.inline;
    if (inline) {
      return (
        <code
          className={cn(
            "relative rounded bg-gray-800/20 px-[0.3rem] py-[0.2rem] font-mono text-sm font-medium text-gray-100",
            className
          )}
          {...props}
        />
      );
    }
    
    return (
      <code
        className={cn(
          "relative rounded font-mono text-sm font-medium text-gray-100",
          className
        )}
        {...props}
      />
    );
  },
  
  pre: ({ className, ...props }) => (
    <pre
      className={cn(
        "mt-4 mb-4 overflow-x-auto rounded-lg bg-gray-900 p-4 text-gray-100",
        className
      )}
      {...props}
    />
  ),
  
  td: ({ className, ...props }) => (
    <td
      className={cn(
        "border border-gray-600 px-4 py-2 text-gray-200 [&[align=center]]:text-center [&[align=right]]:text-right",
        className
      )}
      {...props}
    />
  ),
  
  th: ({ className, ...props }) => (
    <th
      className={cn(
        "border border-gray-600 px-4 py-2 text-left font-semibold text-white [&[align=center]]:text-center [&[align=right]]:text-right",
        className
      )}
      {...props}
    />
  ),
};
