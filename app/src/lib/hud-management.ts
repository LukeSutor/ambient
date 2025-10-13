import { HudDimensions, HudSizeOption } from "@/types/settings";
import { invoke } from "@tauri-apps/api/core";
import { SettingsService } from "./settings-service";

export class HudManagement {
    static isExpanded: boolean = false;

    static async collapseChatWindow() {
        const dimensions: HudDimensions = await SettingsService.getHudDimensions();

        await invoke("resize_hud", {
            width: dimensions.width,
            height: dimensions.input_bar_height,
        });
        this.isExpanded = false;
    }

    //TODO: Add other functions for managing HUD state
}