"use client";

import { Switch } from "@/components/ui/switch";
import { useRoleAccess } from "@/lib/role-access";
import { useSettings } from "@/lib/settings";
import type { SkillSummary } from "@/types/skills";
import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";

/** User-friendly descriptions for each skill */
const SKILL_DESCRIPTIONS: Record<string, string> = {
  "web-search":
    "Search the web for up-to-date information, news, and answers to your questions",
  "memory-search": "Recall information from your past conversations",
  "code-execution":
    "Run code snippets to perform calculations, data processing, and more",
  calendar: "View, create, and manage events on your Google Calendar",
  email: "Read and search emails using your Gmail account",
  "automation-management": "Create and manage automations to complete tasks on your computer",
};

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
 * Get user-friendly description for a skill, falling back to the model-facing one
 */
function getSkillDescription(skill: SkillSummary): string {
  return SKILL_DESCRIPTIONS[skill.name] ?? skill.description;
}

/**
 * Determine why a skill is locked, following the hierarchy:
 * requires_online > requires_auth > requires_google_auth
 *
 * Returns a reason string if locked, or null if the skill is available.
 */
function getLockedReason(
  skill: SkillSummary,
  isOnline: boolean,
  isLoggedIn: boolean,
  isGoogleAuthenticated: boolean,
): string | null {
  if (skill.requires_online && !isOnline) {
    return "Requires internet connection";
  }
  if (skill.requires_auth && !isLoggedIn) {
    return "Requires sign-in";
  }
  if (skill.requires_google_auth && !isGoogleAuthenticated) {
    return "Requires Google sign-in";
  }
  return null;
}

/**
 * SkillToggles component displays a list of available tools
 * with toggle switches to enable/disable them for the agentic runtime.
 */
export function SkillToggles() {
  const { settings, toggleSkill } = useSettings();
  const {
    isOnline,
    isLoggedIn,
    isGoogleAuthenticated,
    refresh: refreshAuth,
  } = useRoleAccess();
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadSkills = async () => {
      try {
        // Refresh auth state to ensure Google auth status is current
        await refreshAuth();
        const availableSkills = await invoke<SkillSummary[]>(
          "get_available_skills",
        );
        console.log("Available skills:", availableSkills);
        setSkills(availableSkills);
      } catch (error) {
        console.error("[SkillToggles] Failed to load skills:", error);
      } finally {
        setLoading(false);
      }
    };
    void loadSkills();
  }, [refreshAuth]);

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
      <div className="p-4 text-sm text-muted-foreground">Loading tools...</div>
    );
  }

  if (skills.length === 0) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        No tools available
      </div>
    );
  }

  const disabledSkills = settings?.disabled_skills ?? [];

  return (
    <div className="flex flex-col divide-y divide-border">
      {skills.map((skill) => {
        const isEnabled = !disabledSkills.includes(skill.name);
        const lockedReason = getLockedReason(
          skill,
          isOnline,
          isLoggedIn,
          isGoogleAuthenticated,
        );
        const isLocked = lockedReason !== null;

        return (
          <div
            key={`skill-toggle-${skill.name}`}
            className="flex flex-row items-center justify-between p-4"
          >
            <div className="flex flex-col gap-0.5 pr-4">
              <div className="flex items-center gap-2">
                <p className="font-semibold text-sm">
                  {formatSkillName(skill.name)}
                </p>
                {isLocked && (
                  <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
                    {lockedReason}
                  </span>
                )}
              </div>
              <p className="text-sm text-muted-foreground">
                {getSkillDescription(skill)}
              </p>
            </div>
            <Switch
              checked={isEnabled && !isLocked}
              disabled={isLocked}
              onCheckedChange={() => void handleToggle(skill.name)}
            />
          </div>
        );
      })}
    </div>
  );
}
