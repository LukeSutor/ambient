"use client";

import { Button } from "@/components/ui/button";
import { Crown, Sparkles, Zap, Brain, ArrowLeft } from "lucide-react";
import { useRouter } from "next/navigation";

const PRO_FEATURES = [
  {
    icon: Zap,
    title: "Unlimited Cloud Requests",
    description: "No daily limits on Gemini Flash and Pro models.",
  },
  {
    icon: Brain,
    title: "Gemini 3 Pro Access",
    description: "Google's most powerful reasoning model for complex tasks.",
  },
  {
    icon: Sparkles,
    title: "Priority Support",
    description: "Get help faster with dedicated priority support.",
  },
];

export default function UpgradePage() {
  const router = useRouter();

  return (
    <div className="max-w-2xl mx-auto py-8">
      {/* Back button */}
      <button
        onClick={() => router.back()}
        className="flex items-center gap-1.5 text-sm text-muted-foreground hover:text-foreground transition-colors mb-8"
        type="button"
      >
        <ArrowLeft className="h-4 w-4" />
        Back
      </button>

      {/* Header */}
      <div className="text-center mb-10">
        <div className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-purple-500 to-pink-500 mb-4">
          <Crown className="h-8 w-8 text-white" />
        </div>
        <h1 className="text-3xl font-bold tracking-tight mb-2">
          Upgrade to Pro
        </h1>
        <p className="text-muted-foreground text-lg">
          Unlock the full power of Ambient with unlimited cloud models.
        </p>
      </div>

      {/* Features */}
      <div className="space-y-4 mb-10">
        {PRO_FEATURES.map((feature) => (
          <div
            key={feature.title}
            className="flex items-start gap-4 p-4 rounded-xl border bg-card"
          >
            <div className="flex items-center justify-center w-10 h-10 rounded-lg bg-purple-100 shrink-0">
              <feature.icon className="h-5 w-5 text-purple-600" />
            </div>
            <div>
              <h3 className="font-semibold">{feature.title}</h3>
              <p className="text-sm text-muted-foreground">
                {feature.description}
              </p>
            </div>
          </div>
        ))}
      </div>

      {/* CTA */}
      <div className="text-center space-y-4">
        <div className="inline-block rounded-2xl border-2 border-dashed border-purple-200 bg-purple-50/50 px-8 py-6">
          <p className="text-sm font-medium text-purple-700 mb-1">
            Coming Soon
          </p>
          <p className="text-xs text-purple-500">
            Pro subscriptions are not yet available. Stay tuned!
          </p>
        </div>

        <div>
          <Button
            disabled
            size="lg"
            className="bg-gradient-to-r from-purple-500 to-pink-500 text-white hover:from-purple-600 hover:to-pink-600 px-8"
          >
            <Sparkles className="h-4 w-4 mr-2" />
            Subscribe to Pro
          </Button>
        </div>
      </div>
    </div>
  );
}
