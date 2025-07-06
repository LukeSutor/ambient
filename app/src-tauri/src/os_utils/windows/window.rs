use std::result::Result;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use windows::{
  core::*,
  Win32::{
    System::{
      Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
      },
      Variant::{VARIANT},
    },
    UI::Accessibility::{
      CUIAutomation, IUIAutomation, IUIAutomationValuePattern, TreeScope_Descendants,
      TreeScope_Children, UIA_NamePropertyId, UIA_ValuePatternId, UIA_WindowControlTypeId,
      UIA_ControlTypePropertyId,
    },
  },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationTextData {
  pub application_name: String,
  pub window_title: String,
  pub text_content: Vec<String>,
}

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
      let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
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
      let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
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

      let count = elements
        .Length()
        .map_err(|e| format!("Failed to get element count: {:?}", e))?;
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
      let name_variant = VARIANT::from(name.clone());
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

#[tauri::command]
pub fn get_screen_text_by_application() -> Result<Vec<ApplicationTextData>, String> {
  println!("get_screen_text_by_application called");
  unsafe {
    let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    if hr.is_err() {
      return Err(format!("CoInitializeEx failed: {:?}", hr));
    }

    let result = (|| {
      let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

      let root = automation
        .GetRootElement()
        .map_err(|e| format!("Failed to get root element: {:?}", e))?;
      
      println!("Root element obtained");

      // Create condition for window control type
      let window_type_variant = VARIANT::from(UIA_WindowControlTypeId.0 as i32);
      let window_condition = automation
        .CreatePropertyCondition(UIA_ControlTypePropertyId, &window_type_variant)
        .map_err(|e| format!("Failed to create window condition: {:?}", e))?;
      
      println!("Window condition created");

      // Find all top-level windows
      let windows = root
        .FindAll(TreeScope_Children, &window_condition)
        .map_err(|e| format!("Failed to find windows: {:?}", e))?;
      
      println!("Found windows");

      let window_count = windows
        .Length()
        .map_err(|e| format!("Failed to get window count: {:?}", e))?;

      println!("Found {} windows", window_count);

      let mut applications_data = Vec::new();

      for i in 0..window_count {
        let window = windows
          .GetElement(i)
          .map_err(|e| format!("Failed to get window {}: {:?}", i, e))?;

        // Skip if window is not visible
        if let Ok(is_offscreen) = window.CurrentIsOffscreen() {
          if is_offscreen.as_bool() {
            continue;
          }
        }

        // Get window title
        let window_title = match window.CurrentName() {
          Ok(name_bstr) => {
            let title = name_bstr.to_string();
            if title.trim().is_empty() {
              continue; // Skip windows without titles
            }
            title
          }
          Err(_) => continue, // Skip windows we can't get names for
        };

        // Try to get the application name (process name)
        let application_name = match window.CurrentProcessId() {
          Ok(pid) => {
            // Try to get process name from PID
            format!("PID_{}", pid)
          }
          Err(_) => "Unknown".to_string(),
        };

        // Get all text content from this window
        let text_content = get_text_from_window(&automation, &window)?;

        if !text_content.is_empty() {
          applications_data.push(ApplicationTextData {
            application_name,
            window_title,
            text_content,
          });
        }
      }

      Ok(applications_data)
    })();

    CoUninitialize();
    result
  }
}

fn get_text_from_window(
  automation: &IUIAutomation,
  window: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) -> Result<Vec<String>, String> {
  unsafe {
    let condition = automation
      .CreateTrueCondition()
      .map_err(|e| format!("Failed to create true condition: {:?}", e))?;

    let elements = window
      .FindAll(TreeScope_Descendants, &condition)
      .map_err(|e| format!("Failed to find descendants: {:?}", e))?;

    let count = elements
      .Length()
      .map_err(|e| format!("Failed to get element count: {:?}", e))?;

    let mut text_content = Vec::new();

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

      // Try to get text content
      if let Ok(name_bstr) = element.CurrentName() {
        let text = name_bstr.to_string();
        if !text.trim().is_empty() && text.len() > 1 {
          text_content.push(text);
        }
      }

      // Also try to get value if it's an input element
      if let Ok(value_pattern) = element.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
        if let Ok(value_bstr) = value_pattern.CurrentValue() {
          let value = value_bstr.to_string();
          if !value.trim().is_empty() && value.len() > 1 {
            text_content.push(value);
          }
        }
      }
    }

    Ok(text_content)
  }
}
