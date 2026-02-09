"use client";

import { Switch } from "@/components/ui/switch";
import { useSettings } from "@/lib/settings";
import type { SkillSummary } from "@/types/skills";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

/**
 * Format a skill name for display (e.g. "web-search" → "Web Search")
 */
function formatSkillName(name: string): string {
  return name
    .split(/[-_]/)
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");
}

/**
 * SkillToggles component displays a list of available skills
 * with toggle switches to enable/disable them for the agentic runtime.
 */
export function SkillToggles() {
  const { settings, toggleSkill } = useSettings();
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadSkills = async () => {
      try {
        const availableSkills =
          await invoke<SkillSummary[]>("get_available_skills");
        setSkills(availableSkills);
      } catch (error) {
        console.error("[SkillToggles] Failed to load skills:", error);
      } finally {
        setLoading(false);
      }
    };
    void loadSkills();
  }, []);

  const handleToggle = useCallback(
    async (skillName: string) => {
      try {
        await toggleSkill(skillName);
      } catch (error) {
        console.error(
          `[SkillToggles] Failed to toggle skill ${skillName}:`,
          error,
        );
      }
    },
    [toggleSkill],
  );

  if (loading) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        Loading skills...
      </div>
    );
  }

  if (skills.length === 0) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        No skills available
      </div>
    );
  }

  const disabledSkills = settings?.disabled_skills ?? [];

  return (
    <div className="flex flex-col">
      {skills.map((skill, index) => {
        const isEnabled = !disabledSkills.includes(skill.name);

        return (
          <div key={skill.name}>
            <div className="flex flex-row items-center justify-between p-4">
              <div className="flex flex-col gap-0.5 pr-4">
                <p className="font-semibold text-sm">
                  {formatSkillName(skill.name)}
                </p>
                <p className="text-sm text-muted-foreground">
                  {skill.description}
                </p>
              </div>
              <Switch
                checked={isEnabled}
                onCheckedChange={() => void handleToggle(skill.name)}
              />
            </div>
            {index < skills.length - 1 && (
              <div className="border-t border-gray-300" />
            )}
          </div>
        );
      })}
    </div>
  );
}
