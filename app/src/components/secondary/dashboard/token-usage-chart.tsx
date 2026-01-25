"use client";

import { ButtonGroup } from "@/components/ui/button-group";
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  type ChartConfig,
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
} from "@/components/ui/chart";
import { Toggle } from "@/components/ui/toggle";
import type { TimeFilter, TokenUsageQueryResult } from "@/types/token_usage";
import { ChartColumn } from "lucide-react";
import { useState } from "react";
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { TimeFilterButtons } from "./time-filter-buttons";

const chartConfig = {
  local: {
    label: "Local",
    color: "#10b981",
  },
  fast: {
    label: "Fast",
    color: "#60a5fa",
  },
  pro: {
    label: "Pro",
    color: "#2563eb",
  },
  "computer-use": {
    label: "Computer Use",
    color: "#f59e0b",
  },
} satisfies ChartConfig;

interface TokenUsageChartProps {
  chartData: TokenUsageQueryResult | null;
  timeFilter: TimeFilter;
  onTimeFilterChange: (filter: TimeFilter) => void;
}

export function TokenUsageChart({
  chartData,
  timeFilter,
  onTimeFilterChange,
}: TokenUsageChartProps) {
  const [logScale, setLogScale] = useState(false);

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle>Token Usage Overview</CardTitle>
        <CardDescription>{chartData?.time_range}</CardDescription>
        <CardAction>
          <ButtonGroup>
            <TimeFilterButtons
              currentFilter={timeFilter}
              onFilterChange={onTimeFilterChange}
            />
            <ButtonGroup>
              <Toggle
                pressed={logScale}
                onPressedChange={setLogScale}
                aria-label="Toggle log scale"
                variant="outline"
                className="data-[state=on]:bg-gray-100 data-[state=on]:*:[svg]:stroke-blue-500"
              >
                <ChartColumn />
                Log Scale
              </Toggle>
            </ButtonGroup>
          </ButtonGroup>
        </CardAction>
      </CardHeader>
      <CardContent>
        <ChartContainer config={chartConfig}>
          <BarChart accessibilityLayer data={chartData?.data || []}>
            <CartesianGrid vertical={false} />
            <XAxis dataKey="time_label" tickLine={false} axisLine={false} />
            <YAxis
              scale={logScale ? "log" : "linear"}
              domain={[1, "auto"]}
              tickLine={false}
              axisLine={false}
            />
            <ChartTooltip
              cursor={false}
              content={<ChartTooltipContent indicator="dot" />}
            />
            <ChartLegend content={<ChartLegendContent />} />
            {chartData?.models.map((model) => (
              <Bar
                key={model}
                dataKey={model}
                fill={
                  chartConfig[model as keyof typeof chartConfig].color || "gray"
                }
                radius={4}
              />
            ))}
          </BarChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
}
