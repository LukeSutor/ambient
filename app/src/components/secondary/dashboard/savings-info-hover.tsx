"use client";

import { Button } from "@/components/ui/button";
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card";
import { open } from "@tauri-apps/plugin-shell";
import { DollarSign, Droplet, Info, Zap } from "lucide-react";

const PRICING_SOURCES = {
  gemini: "https://ai.google.dev/gemini-api/docs/pricing",
  google:
    "https://cloud.google.com/blog/products/infrastructure/measuring-the-environmental-impact-of-ai-inference",
};

interface SourceLinkProps {
  url: string;
  children: React.ReactNode;
}

function SourceLink({ url, children }: SourceLinkProps) {
  const openURL = async () => {
    await open(url);
  };

  return (
    <Button
      className="p-0 mx-1.5 h-min"
      variant="link"
      onClick={() => void openURL()}
    >
      {children}
    </Button>
  );
}

interface InfoItemProps {
  icon: React.ReactNode;
  children: React.ReactNode;
}

function InfoItem({ icon, children }: InfoItemProps) {
  return (
    <div className="flex flex-row items-center space-x-2 p-2 shadow rounded-md ring-1 ring-gray-300 hover:scale-101 transition-all">
      {icon}
      <p className="font-l">{children}</p>
    </div>
  );
}

export function SavingsInfoHover() {
  return (
    <HoverCard>
      <HoverCardTrigger>
        <Info className="w-4 h-4 text-gray-500 cursor-pointer" />
      </HoverCardTrigger>
      <HoverCardContent side="bottom" align="center" className="text-sm w-86">
        <p>
          This section displays the total cost, water, and energy savings
          achieved by using local AI models instead of cloud-based models.
        </p>
        <div className="flex flex-col items-center space-y-4 mt-4 mb-2">
          <InfoItem
            icon={
              <DollarSign className="text-green-600 h-6 w-6 flex-shrink-0" />
            }
          >
            It costs about $4.34 per million tokens to use
            <SourceLink url={PRICING_SOURCES.gemini}>Gemini</SourceLink>
          </InfoItem>

          <InfoItem
            icon={<Droplet className="text-blue-600 h-6 w-6 flex-shrink-0" />}
          >
            Gemini uses about 0.26 liters of water per response
            <SourceLink url={PRICING_SOURCES.google}>
              according to Google
            </SourceLink>
          </InfoItem>

          <InfoItem
            icon={<Zap className="text-yellow-400 h-6 w-6 flex-shrink-0" />}
          >
            Gemini uses about 0.24 Wh of electricity per response
            <SourceLink url={PRICING_SOURCES.google}>
              according to Google
            </SourceLink>
          </InfoItem>
        </div>
      </HoverCardContent>
    </HoverCard>
  );
}
