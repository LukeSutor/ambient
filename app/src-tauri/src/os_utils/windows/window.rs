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
                CoCreateInstance(&windows::Win32::UI::Accessibility::CUIAutomation, None, CLSCTX_INPROC_SERVER)
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