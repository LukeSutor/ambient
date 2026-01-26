"use client";

import { Card, CardContent, CardHeader } from "@/components/ui/card";

interface ConsumptionCardProps {
  label: string;
  value: number | undefined;
  unit: string | undefined;
}

export function ConsumptionCard({ label, value, unit }: ConsumptionCardProps) {
  return (
    <Card>
      <CardHeader className="text-sm">{label}</CardHeader>
      <CardContent className="flex flex-row items-baseline justify-center mt-auto">
        <p className="text-2xl lg:text-4xl text-black font-bold mr-1">
          {value?.toFixed(2)}
        </p>
        <p className="text-base lg:text-xl">{unit}</p>
      </CardContent>
    </Card>
  );
}
