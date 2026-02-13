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
import type { ModelEntry } from "@/types/models";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { ChartColumn } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts";
import { TimeFilterButtons } from "./time-filter-buttons";

export function TokenUsageChart() {
  const [chartData, setChartData] = useState<TokenUsageQueryResult | null>(
    null,
  );
  const [timeFilter, setTimeFilter] = useState<TimeFilter>("Last7Days");
  const [logScale, setLogScale] = useState(false);
  const [models, setModels] = useState<ModelEntry[]>([]);

  // Build chart config from DB models
  const chartConfig = useMemo(() => {
    const config: ChartConfig = {};
    for (const model of models) {
      config[model.model] = {
        label: model.display_name,
        color: model.color,
      };
    }
    // Fallback if models haven't loaded yet
    if (Object.keys(config).length === 0) {
      return {
        "qwen3vl-2b": { label: "Local", color: "#10b981" },
        "gemini-3-flash": { label: "Gemini 3 Flash", color: "#60a5fa" },
        "gemini-3-pro": { label: "Gemini 3 Pro", color: "#2563eb" },
      } satisfies ChartConfig;
    }
    return config;
  }, [models]);

  const fetchModels = useCallback(async () => {
    try {
      const result = await invoke<ModelEntry[]>("get_models");
      setModels(result);
    } catch (error) {
      console.error("Failed to fetch models:", error);
    }
  }, []);

  const fetchChartData = useCallback(async () => {
    try {
      const data = await invoke<TokenUsageQueryResult>("get_token_usage", {
        timeFilter,
      });
      setChartData(data);
    } catch (error) {
      console.error("Failed to fetch token usage:", error);
    }
  }, [timeFilter]);

  useEffect(() => {
    void fetchModels();
  }, [fetchModels]);

  useEffect(() => {
    void fetchChartData();
  }, [fetchChartData]);

  useEffect(() => {
    const unlisten = listen("token_usage_changed", () => {
      void fetchChartData();
    });

    return () => {
      void unlisten.then((fn) => {
        fn();
      });
    };
  }, [fetchChartData]);

  return (
    <Card className="w-full">
      <CardHeader>
        <CardTitle>Token Usage Overview</CardTitle>
        <CardDescription>{chartData?.time_range}</CardDescription>
        <CardAction>
          <ButtonGroup>
            <TimeFilterButtons
              currentFilter={timeFilter}
              onFilterChange={setTimeFilter}
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
                fill={chartConfig[model]?.color || "gray"}
                radius={4}
              />
            ))}
          </BarChart>
        </ChartContainer>
      </CardContent>
    </Card>
  );
}
