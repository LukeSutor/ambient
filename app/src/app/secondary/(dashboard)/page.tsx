"use client";

import Link from "next/link";

export default function Home() {
  return (
    <div>
      <Link href="/secondary/signin">Go to Secondary Sign In</Link>
      <h1>Secondary Window</h1>
    </div>
  );
}
