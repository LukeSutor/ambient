use std::result::Result;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::{
    core::*,
    Win32::{
        System::{
            Variant::{VARIANT, VT_BSTR},
            Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, CLSCTX_INPROC_SERVER},
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationValuePattern,
            TreeScope_Descendants, UIA_NamePropertyId, UIA_ValuePatternId,
        },
    },
};

#[tauri::command]
pub fn get_focused_window_name() -> Result<String, String> {
    unsafe {
        // Initialize COM
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("CoInitializeEx failed: {:?}", hr));
        }

        let result = (|| {
            // Create UIAutomation instance
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

            // Get focused element
            let element = automation
                .GetFocusedElement()
                .map_err(|e| format!("Failed to get focused element: {:?}", e))?;

            // Get the element's name
            let name_bstr = element
                .CurrentName()
                .map_err(|e| format!("Failed to get element name: {:?}", e))?;

            let name_str = name_bstr.to_string();

            Ok(name_str)
        })();

        CoUninitialize();

        result
    }
}


#[tauri::command]
pub fn get_all_text_from_focused_app() -> Result<String, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("CoInitializeEx failed: {:?}", hr));
        }

        let result = (|| {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

            let mut root = automation
                .GetFocusedElement()
                .map_err(|e| format!("Failed to get focused element: {:?}", e))?;

            // Climb to the topmost parent (window)
            loop {
                match root.GetCachedParent() {
                    Ok(parent) => root = parent,
                    _ => break, // no more parents
                }
            }

            let condition = automation
                .CreateTrueCondition()
                .map_err(|e| format!("Failed to get TrueCondition: {:?}", e))?;

            let elements = root
                .FindAll(TreeScope_Descendants, &condition)
                .map_err(|e| format!("Failed to find descendants: {:?}", e))?;

            let count = elements.Length().map_err(|e| format!("Failed to get element count: {:?}", e))?;
            let mut visible_texts = vec![];

            for i in 0..count {
                let element = elements
                    .GetElement(i)
                    .map_err(|e| format!("Failed to get element {}: {:?}", i, e))?;

                // Skip elements that are offscreen
                if let Ok(offscreen) = element.CurrentIsOffscreen() {
                    if offscreen.as_bool() {
                        continue;
                    }
                }

                // Try to get name (static text or labels)
                if let Ok(name_bstr) = element.CurrentName() {
                    let name = name_bstr.to_string();
                    if !name.trim().is_empty() {
                        visible_texts.push(name);
                        continue;
                    }
                }
            }

            Ok(visible_texts.join("\n"))
        })();

        CoUninitialize();

        result
    }
}


#[tauri::command]
pub fn get_brave_url() -> Result<String, String> {
    unsafe {
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("CoInitializeEx failed: {:?}", hr));
        }

        let result = (|| {
            // Step 1: Create automation instance
            let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

            // Step 2: Get the root (desktop)
            let root = automation
                .GetRootElement()
                .map_err(|e| format!("Failed to get desktop root: {:?}", e))?;

            // Step 3: Create a condition for the name "Address and search bar"
            let name = BSTR::from("Address and search bar");
            let mut name_variant = VARIANT::from(name.clone());
            // name_variant.vt = VT_BSTR;
            // name_variant.bstrVal = name;

            let name_condition = automation
                .CreatePropertyCondition(UIA_NamePropertyId, &name_variant)
                .map_err(|e| format!("Failed to create property condition: {:?}", e))?;

            // Step 4: Search for the first matching element
            let address_bar = root
                .FindFirst(TreeScope_Descendants, &name_condition)
                .map_err(|e| format!("FindFirst failed: {:?}", e))?;

            // Step 5: Get the ValuePattern and extract the URL
            let value_pattern: IUIAutomationValuePattern = address_bar
                .GetCurrentPatternAs(UIA_ValuePatternId)
                .map_err(|e| format!("Failed to get ValuePattern: {:?}", e))?;

            let url_bstr = value_pattern
                .CurrentValue()
                .map_err(|e| format!("Failed to get current value: {:?}", e))?;

            Ok(url_bstr.to_string())
        })();

        CoUninitialize();
        result
    }
}