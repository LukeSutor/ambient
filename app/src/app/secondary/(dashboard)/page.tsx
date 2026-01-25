"use client";

import {
  ConsumptionCard,
  SavingsInfoHover,
  TokenUsageChart,
} from "@/components/secondary/dashboard";
import { useRoleAccess } from "@/lib/role-access";
import type {
  TimeFilter,
  TokenUsageConsumptionResult,
  TokenUsageQueryResult,
} from "@/types/token_usage";
import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";

const GREETINGS = [
  "Hello",
  "Welcome back",
  "Good to see you",
  "Hi there",
  "Howdy",
];

function getTimeGreeting(): string {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}

function getRandomGreeting(): string {
  if (Math.random() < 0.5) {
    return getTimeGreeting();
  }
  return GREETINGS[Math.floor(Math.random() * GREETINGS.length)];
}

export default function Home() {
  const [greeting, setGreeting] = useState<string>("");
  const [consumptionData, setConsumptionData] =
    useState<TokenUsageConsumptionResult | null>(null);
  const [chartData, setChartData] = useState<TokenUsageQueryResult | null>(
    null,
  );
  const [timeFilter, setTimeFilter] = useState<TimeFilter>("Last7Days");

  const { userInfo, getFirstName } = useRoleAccess();

  // Set greeting when user info is available
  useEffect(() => {
    if (userInfo) {
      setGreeting(getRandomGreeting());
    }
  }, [userInfo]);

  // Fetch consumption data
  useEffect(() => {
    const fetchConsumptionData = async () => {
      const data = await invoke<TokenUsageConsumptionResult>(
        "get_token_usage_consumption",
      );
      setConsumptionData(data);
    };
    void fetchConsumptionData();
  }, []);

  // Fetch chart data when time filter changes
  useEffect(() => {
    const fetchChartData = async () => {
      const data = await invoke<TokenUsageQueryResult>("get_token_usage", {
        timeFilter,
      });
      setChartData(data);
    };
    void fetchChartData();
  }, [timeFilter]);

  return (
    <div className="relative flex flex-col items-center justify-start p-4 w-full">
      {/* Greeting */}
      {userInfo ? (
        <p className="text-4xl font-bold w-full h-20 font-sora">
          {greeting}, {getFirstName()}
        </p>
      ) : (
        <div className="h-20 w-full" />
      )}

      {/* Consumption cards header */}
      <div className="flex flex-row items-center justify-start space-x-2 w-full">
        <p className="text-xl font-medium">Total Savings</p>
        <SavingsInfoHover />
      </div>

      {/* Consumption cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4 w-full my-4">
        <ConsumptionCard
          label="Cost"
          value={consumptionData?.cost_amount}
          unit={consumptionData?.cost_unit}
        />
        <ConsumptionCard
          label="Water"
          value={consumptionData?.water_amount}
          unit={consumptionData?.water_unit}
        />
        <ConsumptionCard
          label="Energy"
          value={consumptionData?.energy_amount}
          unit={consumptionData?.energy_unit}
        />
      </div>

      {/* Token usage graph */}
      <TokenUsageChart
        chartData={chartData}
        timeFilter={timeFilter}
        onTimeFilterChange={setTimeFilter}
      />
    </div>
  );
}
