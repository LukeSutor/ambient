"use client";

import Link from "next/link";

interface AuthFooterProps {
  text: string;
  linkText: string;
  linkHref: string;
}

export function AuthFooter({ text, linkText, linkHref }: AuthFooterProps) {
  return (
    <div className="text-center">
      <p className="text-sm text-gray-600">
        {text}{" "}
        <Link
          href={linkHref}
          className="font-medium text-blue-600 hover:text-blue-500 transition-colors"
        >
          {linkText}
        </Link>
      </p>
    </div>
  );
}
