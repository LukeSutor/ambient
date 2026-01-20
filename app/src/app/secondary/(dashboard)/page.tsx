"use client";

import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useRoleAccess } from "@/lib/role-access";
import {
  Card,
  CardAction,
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
import {
  HoverCard,
  HoverCardContent,
  HoverCardTrigger,
} from "@/components/ui/hover-card"
import { Toggle } from "@/components/ui/toggle"
import { Bar, BarChart, CartesianGrid, XAxis, YAxis } from "recharts"
import { TimeFilter, AggregationLevel, TokenUsageQueryResult, TokenUsageConsumptionResult } from "@/types/token_usage";
import { ChartColumn, DollarSign, Droplet, ExternalLink, Info, Zap } from "lucide-react";
import { open } from "@tauri-apps/plugin-shell";
import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group"
import { set } from "react-hook-form";

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
    async function fetchData() {
      const data = await invoke<TokenUsageQueryResult>('get_token_usage', { timeFilter });
      setChartData(data);
    }
    fetchData();
  }, [timeFilter]);

  const openURL = async (url: string) => {
    await open(url);
  }

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      {/* Greeting */}
      {userInfo ? 
        <p className="text-4xl font-bold w-full h-20 font-sora">{greeting}{", "}{getFirstName()}</p>
        :
        <div className="h-20 w-full" />
      }
      {/* Consumption cards */}
      <div className="flex flex-row items-center justify-start space-x-2 w-full">
        <p className="text-xl font-medium">Total Savings</p>
        <HoverCard>
          <HoverCardTrigger>
            <Info className="w-4 h-4 text-gray-500 cursor-pointer" />
          </HoverCardTrigger>
          <HoverCardContent side="bottom" align="center" className="text-sm w-86">
            <p>This section displays the total cost, water, and energy savings achieved by using local AI models instead of cloud-based models.</p>
            <div className="flex flex-col items-center space-y-4 mt-4 mb-2">
              <div className="flex flex-row items-center space-x-2 p-2 shadow rounded-md ring-1 ring-gray-300 hover:scale-101 transition-all">
                <DollarSign className="text-green-600 h-6 w-6 flex-shrink-0" />
                <p className="font-l">It costs about $4.34 per million tokens to use
                  <Button className="p-0 mx-1.5 h-min" variant="link" onClick={() => openURL("https://ai.google.dev/gemini-api/docs/pricing")}>Gemini</Button>
                </p>
              </div>
              <div className="flex flex-row items-center space-x-2 p-2 shadow rounded-md ring-1 ring-gray-300 hover:scale-101 transition-all">
                <Droplet className="text-blue-600 h-6 w-6 flex-shrink-0" />
                <p className="font-l">Gemini uses about 0.26 liters of water per response
                  <Button className="p-0 mx-1.5 h-min" variant="link" onClick={() => openURL("https://cloud.google.com/blog/products/infrastructure/measuring-the-environmental-impact-of-ai-inference")}>according to Google</Button>
                </p>
              </div>
              <div className="flex flex-row items-center space-x-2 p-2 shadow rounded-md ring-1 ring-gray-300 hover:scale-101 transition-all">
                <Zap className="text-yellow-400 h-6 w-6 flex-shrink-0" />
                <p className="font-l">Gemini uses about 0.24 Wh of electricity per response
                  <Button className="p-0 mx-1.5 h-min" variant="link" onClick={() => openURL("https://cloud.google.com/blog/products/infrastructure/measuring-the-environmental-impact-of-ai-inference")}>according to Google</Button>
                </p>
              </div>
            </div>
          </HoverCardContent>
        </HoverCard>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 w-full my-4">
        <Card>
          <CardHeader className="text-sm">Cost</CardHeader>
          <CardContent className="flex flex-row items-baseline justify-center mt-auto">
            <p className="text-4xl text-black font-bold mr-1">{consumptionData?.cost_amount?.toFixed(2)}</p>
            <p className="text-xl">{consumptionData?.cost_unit}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="text-sm block">Water</CardHeader>
          <CardContent className="flex flex-row items-baseline justify-center mt-auto">
            <p className="text-4xl text-black font-bold mr-1">{consumptionData?.water_amount?.toFixed(2)}</p>
            <p className="text-xl">{consumptionData?.water_unit}</p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="text-sm">Energy</CardHeader>
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
          <CardAction>
            <ButtonGroup>
              <ButtonGroup>
                <Button className={`${timeFilter === "Last3Months" ? "bg-gray-100" : ""}`} onClick={() => setTimeFilter("Last3Months")} variant="outline">Last 3 Months</Button>
                <Button className={`${timeFilter === "Last30Days" ? "bg-gray-100" : ""}`} onClick={() => setTimeFilter("Last30Days")} variant="outline">Last 30 Days</Button>
                <Button className={`${timeFilter === "Last7Days" ? "bg-gray-100" : ""}`} onClick={() => setTimeFilter("Last7Days")} variant="outline">Last 7 Days</Button>
              </ButtonGroup>
              <ButtonGroup>
                <Toggle
                  pressed={logScale}
                  onPressedChange={(pressed) => setLogScale(pressed)}
                  aria-label="Toggle bookmark"
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
              <XAxis
                dataKey="time_label"
                tickLine={false}
                axisLine={false}
              />
              <YAxis
                scale={logScale ? "log" : "linear"}
                domain={[1, "auto"]}
                tickLine={false}
                axisLine={false}
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
      </Card>
    </div>
  );
}
