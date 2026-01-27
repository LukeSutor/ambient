"use client";

import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import type { TimeFilter } from "@/types/token_usage";
import { ChevronDown } from "lucide-react";

interface TimeFilterButtonsProps {
  currentFilter: TimeFilter;
  onFilterChange: (filter: TimeFilter) => void;
}

const TIME_FILTERS: { value: TimeFilter; label: string }[] = [
  { value: "Last3Months", label: "Last 3 Months" },
  { value: "Last30Days", label: "Last 30 Days" },
  { value: "Last7Days", label: "Last 7 Days" },
];

export function TimeFilterButtons({
  currentFilter,
  onFilterChange,
}: TimeFilterButtonsProps) {
  const currentLabel =
    TIME_FILTERS.find((f) => f.value === currentFilter)?.label || currentFilter;

  return (
    <>
      {/* Mobile/Tablet Dropdown */}
      <div className="lg:hidden">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline">
              {currentLabel}
              <ChevronDown className="ml-2 h-4 w-4 opacity-50" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {TIME_FILTERS.map(({ value, label }) => (
              <DropdownMenuItem
                key={value}
                onClick={() => {
                  onFilterChange(value);
                }}
                className={currentFilter === value ? "bg-gray-100" : ""}
              >
                {label}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </div>

      {/* Desktop Button Group */}
      <div className="hidden lg:block">
        <ButtonGroup>
          {TIME_FILTERS.map(({ value, label }) => (
            <Button
              key={value}
              className={currentFilter === value ? "bg-gray-100" : ""}
              onClick={() => {
                onFilterChange(value);
              }}
              variant="outline"
            >
              {label}
            </Button>
          ))}
        </ButtonGroup>
      </div>
    </>
  );
}
