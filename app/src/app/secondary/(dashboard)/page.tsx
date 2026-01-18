"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Link from "next/link";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  ChartContainer,
  ChartLegend,
  ChartLegendContent,
  ChartTooltip,
  ChartTooltipContent,
  type ChartConfig,
} from "@/components/ui/chart"
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Bar, BarChart, CartesianGrid, XAxis } from "recharts"
import { TimeFilter, AggregationLevel, TokenUsageQueryResult } from "@/types/token_usage";

const chartConfig = {
  local: {
    label: "Local",
    color: "blue",
  },
  fast: {
    label: "Fast",
    color: "red",
  },
} satisfies ChartConfig

export default function Home() {
  const [chartData, setChartData] = useState<TokenUsageQueryResult | null>(null);
  const [timeFilter, setTimeFilter] = useState<TimeFilter>("Last7Days");
  const [aggregationLevel, setAggregationLevel] = useState<AggregationLevel>("Day");

  useEffect(() => {
    console.log({timeFilter, aggregationLevel})
    async function fetchData() {
      const data = await invoke<TokenUsageQueryResult>('get_token_usage', { timeFilter, aggregationLevel });
      setChartData(data);
      console.log({data})
    }
    fetchData();
  }, [timeFilter, aggregationLevel]);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      <Card className="w-full">
        <CardHeader>
          <CardTitle>Token Usage Overvasdfsafdsadfiew</CardTitle>
          <CardDescription>
            {chartData?.time_range}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <ChartContainer config={chartConfig}>
            <BarChart accessibilityLayer data={chartData?.data || []}>
              <CartesianGrid vertical={false} />
              <XAxis
                dataKey="time_label"
                tickLine={false}
                axisLine={false}
              />
              <ChartTooltip cursor={false} content={<ChartTooltipContent indicator="dot"  />} />
              <ChartLegend content={<ChartLegendContent />} />
              {chartData?.models.map((model, index) => (
                <Bar 
                  key={model} 
                  dataKey={model} 
                  fill={`blue`} 
                  radius={4} 
                />
              ))}
            </BarChart>
          </ChartContainer>
        </CardContent>
        <CardFooter>
          {/* Dropdowns for time filter and aggregation level */}
          <div className="flex flex-row space-x-4">
            <Select value={timeFilter} onValueChange={(value) => setTimeFilter(value as TimeFilter)}>
              <SelectTrigger className="">
                <SelectValue placeholder="Time" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Time</SelectLabel>
                    {(["Last24Hours", "Last7Days", "Last30Days", "LastYear", "AllTime"] as TimeFilter[]).map((filter) => (
                    <SelectItem key={filter} value={filter}>
                      {filter.replace(/([A-Z]|\d+)/g, ' $1').trim()}
                    </SelectItem>
                    ))}
                </SelectGroup>
              </SelectContent>
            </Select>
            <Select value={aggregationLevel} onValueChange={(value) => setAggregationLevel(value as AggregationLevel)}>
              <SelectTrigger className="">
                <SelectValue placeholder="Aggregation" />
              </SelectTrigger>
              <SelectContent>
                <SelectGroup>
                  <SelectLabel>Aggregation Level</SelectLabel>
                    {(["Hour", "Day", "Week", "Month"] as AggregationLevel[]).map((level) => (
                    <SelectItem key={level} value={level}>
                      {level}
                    </SelectItem>
                    ))}
                </SelectGroup>
              </SelectContent>
            </Select>
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}
