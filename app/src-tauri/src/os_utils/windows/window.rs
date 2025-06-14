use std::result::Result;
use windows::core::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::Accessibility::*;

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
        // Initialize COM
        let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        if hr.is_err() {
            return Err(format!("CoInitializeEx failed: {:?}", hr));
        }

        let result = (|| {
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

            let root = automation
                .GetFocusedElement()
                .map_err(|e| format!("Failed to get focused element: {:?}", e))?;

            let condition = automation
                .CreateTrueCondition()
                .map_err(|e| format!("Failed to get TrueCondition: {:?}", e))?;

            // Get all descendants (deep UI tree)
            let elements = root
                .FindAll(TreeScope_Descendants, &condition)
                .map_err(|e| format!("Failed to find all descendants: {:?}", e))?;

            let count = elements.Length().map_err(|e| format!("Failed to get length: {:?}", e))?;

            let mut texts: Vec<String> = Vec::new();

            for i in 0..count {
                let element = elements
                    .GetElement(i)
                    .map_err(|e| format!("Failed to get element {}: {:?}", i, e))?;

                // Try to get name (like for static text)
                if let Ok(name_bstr) = element.CurrentName() {
                    let name = name_bstr.to_string();
                    if !name.trim().is_empty() {
                        texts.push(name);
                        continue;
                    }
                }
            }

            Ok(texts.join("\n"))
        })();

        CoUninitialize();

        result
    }
}