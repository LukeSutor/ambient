"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRoleAccess } from "@/lib/role-access";
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
import { Toggle } from "@/components/ui/toggle"
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts"
import { TimeFilter, AggregationLevel, TokenUsageQueryResult, TokenUsageConsumptionResult } from "@/types/token_usage";
import { ChartColumn } from "lucide-react";

const greetings = [
  "Hello",
  "Welcome back",
  "Good to see you",
  "Hi there",
  "Howdy",
];

function timeGreeting(): string {
    const hour = new Date().getHours();
    if (hour < 12) {
      return "Good morning";
    } else if (hour < 18) {
      return "Good afternoon";
    } else {
      return "Good evening";
    }
}

const chartConfig = {
  "local": {
    label: "Local",
    color: "#10b981",
  },
  "fast": {
    label: "Fast",
    color: "#60a5fa",
  },
  "pro": {
    label: "Pro",
    color: "#2563eb",
  },
  "computer-use": {
    label: "Computer Use",
    color: "#f59e0b",
  },
} satisfies ChartConfig

export default function Home() {
  const [greeting, setGreeting] = useState<string>("");
  const [consumptionData, setConsumptionData] = useState<TokenUsageConsumptionResult | null>(null);
  const [chartData, setChartData] = useState<TokenUsageQueryResult | null>(null);
  const [timeFilter, setTimeFilter] = useState<TimeFilter>("Last7Days");
  const [aggregationLevel, setAggregationLevel] = useState<AggregationLevel>("Day");
  const [logScale, setLogScale] = useState<boolean>(false);

  const { userInfo, getFirstName } = useRoleAccess();

  // Set greeting
  useEffect(() => {
    if (!userInfo) return;

    // Set the greeting randomly based on greetings or time of day
    let greeting = "";
    if (Math.random() < 0.5) {
      greeting = timeGreeting();
    } else {
      greeting = greetings[Math.floor(Math.random() * greetings.length)];
    }
    setGreeting(greeting);
  }, [userInfo]);

  // Fetch consumption data
  useEffect(() => {
    async function fetchConsumptionData() {
      const data = await invoke<TokenUsageConsumptionResult>('get_token_usage_consumption');
      setConsumptionData(data);
    }
    fetchConsumptionData();
  }, []);  

  // Fetch chart data
  useEffect(() => {
    if (aggregationLevel === "Hour" && timeFilter !== "Last24Hours") {
      setTimeFilter("Last24Hours");
      return;
    } else if (aggregationLevel === "Day" && (timeFilter === "LastYear" || timeFilter === "AllTime")) {
      setTimeFilter("Last30Days");
      return;
    }

    async function fetchData() {
      const data = await invoke<TokenUsageQueryResult>('get_token_usage', { timeFilter, aggregationLevel });
      setChartData(data);
    }
    fetchData();
  }, [timeFilter, aggregationLevel]);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      {/* Greeting */}
      {userInfo ? 
        <p className="text-4xl font-bold w-full h-20 font-sora">{greeting}{", "}{getFirstName()}</p>
        :
        <div className="h-20 w-full" />
      }
      {/* Consumption cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 w-full my-4">
        <Card>
          <CardHeader className="text-sm">Cost Savings</CardHeader>
          <CardContent className="flex flex-row items-baseline justify-center mt-auto">
            <p className="text-4xl text-black font-bold mr-1">{consumptionData?.cost_amount?.toFixed(2)}</p>
            <p className="text-xl">{consumptionData?.cost_unit}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="text-sm">Water Savings</CardHeader>
          <CardContent className="flex flex-row items-baseline justify-center mt-auto">
            <p className="text-4xl text-black font-bold mr-1">{consumptionData?.water_amount?.toFixed(2)}</p>
            <p className="text-xl">{consumptionData?.water_unit}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="text-sm">Energy Savings</CardHeader>
          <CardContent className="flex flex-row items-baseline justify-center mt-auto">
            <p className="text-4xl text-black font-bold mr-1">{consumptionData?.energy_amount?.toFixed(2)}</p>
            <p className="text-xl">{consumptionData?.energy_unit}</p>
          </CardContent>
        </Card>
      </div>
      {/* Token usage graph */}
      <Card className="w-full">
        <CardHeader>
          <CardTitle>Token Usage Overview</CardTitle>
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
              <YAxis
                scale={logScale ? "log" : "linear"}
                domain={[1, "auto"]}
                hide={true}
              />
              <ChartTooltip cursor={false} content={<ChartTooltipContent indicator="dot" />} />
              <ChartLegend content={<ChartLegendContent />} />
              {chartData?.models.map((model) => (
                <Bar 
                  key={model} 
                  dataKey={model} 
                  fill={chartConfig[model as keyof typeof chartConfig]?.color || "gray"} 
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
            <Toggle
              pressed={logScale}
              onPressedChange={(pressed) => setLogScale(pressed)}
              aria-label="Toggle bookmark"
              size="sm"
              variant="outline"
              className="font-normal data-[state=on]:bg-gray-100 data-[state=on]:*:[svg]:stroke-blue-500"
            >
              <ChartColumn />
              Log Scale
            </Toggle>
          </div>
        </CardFooter>
      </Card>
    </div>
  );
}
