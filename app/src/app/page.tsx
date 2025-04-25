"use client";
import { RoundedButton } from "@/components/RoundedButton";
import { invoke } from "@tauri-apps/api/core";
import Image from "next/image";
import { useCallback, useState } from "react";
import { Button } from "@/components/ui/button"

export default function Home() {
  const [greeted, setGreeted] = useState<string | null>(null);
  const greet = useCallback((): void => {
    invoke<string>("greet")
      .then((s) => {
        setGreeted(s);
      })
      .catch((err: unknown) => {
        console.error(err);
      });
  }, []);

  // New function to call the sidecar command
  async function callSidecar() {
    // Replace with actual image path and prompt
    const imagePath = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/sample_images/gmail.png";
    const prompt = "Describe what the user is actively doing on their computer in this image.";
    try {
      console.log(`Calling sidecar with image: ${imagePath}, prompt: ${prompt}`);
      const result = await invoke("call_main_sidecar", { imagePath, prompt });
      console.log("Sidecar response:", result);
      // Handle the successful response string (result)
    } catch (error) {
      console.error("Error calling sidecar:", error);
      // Handle the error string (error)
    }
  }

  // New function to call the sidecar command
  async function callLlamaSidecar() {
    // Replace with actual image path and prompt
    const image = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/sample_images/gmail.png";
    const prompt = "You are a computer screenshot analysis expert. You will be given an screenshot of a person using a computer, and you must accurately and precisely describe what they are currently doing based on the screenshot. Your response should be short and sweet, optimized for creating an embedding for document similarity with other tasks the user does. What is the user doing in this image?";
    const model = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/smol.gguf";
    const mmproj = "C:/Users/Luke/Desktop/coding/local-computer-use/backend/models/mmproj.gguf";
    try {
      console.log(`Calling sidecar with image: ${image}, prompt: ${prompt}`);
      const result = await invoke("call_llama_sidecar", { model, mmproj, image, prompt });
      console.log("Sidecar response:", result);
      // Handle the successful response string (result)
    } catch (error) {
      console.error("Error calling sidecar:", error);
      // Handle the error string (error)
    }
  }

  return (
    <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
      <main className="flex flex-col gap-8 row-start-2 items-center sm:items-start">
        <Button variant="outline" onClick={callLlamaSidecar}>Call Sidecar</Button>
        <Image
          className="dark:invert"
          src="/next.svg"
          alt="Next.js logo"
          width={180}
          height={38}
          priority
        />
        <ol className="list-inside list-decimal text-sm text-center sm:text-left font-[family-name:var(--font-geist-mono)]">
          <li className="mb-2">
            Get started by editing{" "}
            <code className="bg-black/[.05] dark:bg-white/[.06] px-1 py-0.5 rounded font-semibold">
              src/app/page.tsx
            </code>
            .
          </li>
          <li>Save and see your changes instantly.</li>
        </ol>

        <div className="flex flex-col gap-2 items-start">
          <RoundedButton
            onClick={greet}
            title="Call &quot;greet&quot; from Rust"
          />
          <p className="break-words w-md">
            {greeted ?? "Click the button to call the Rust function"}
          </p>
        </div>
      </main>
      <footer className="row-start-3 flex gap-6 flex-wrap items-center justify-center">
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://nextjs.org/learn?utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/file.svg"
            alt="File icon"
            width={16}
            height={16}
          />
          Learn
        </a>
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://vercel.com/templates?framework=next.js&utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/window.svg"
            alt="Window icon"
            width={16}
            height={16}
          />
          Examples
        </a>
        <a
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://nextjs.org?utm_source=create-next-app&utm_medium=appdir-template-tw&utm_campaign=create-next-app"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/globe.svg"
            alt="Globe icon"
            width={16}
            height={16}
          />
          Go to nextjs.org →
        </a>
      </footer>
    </div>
  );
}
