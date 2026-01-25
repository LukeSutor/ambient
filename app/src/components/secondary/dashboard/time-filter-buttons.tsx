"use client";

import { Button } from "@/components/ui/button";
import { ButtonGroup } from "@/components/ui/button-group";
import type { TimeFilter } from "@/types/token_usage";

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
  return (
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
  );
}
