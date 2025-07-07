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
      CUIAutomation, IUIAutomation, IUIAutomationValuePattern, IUIAutomationCacheRequest,
      TreeScope_Descendants, TreeScope_Children, TreeScope_Subtree, UIA_NamePropertyId, 
      UIA_ValuePatternId, UIA_WindowControlTypeId, UIA_ControlTypePropertyId,
      UIA_TextControlTypeId, UIA_EditControlTypeId, UIA_DocumentControlTypeId,
      UIA_IsOffscreenPropertyId,
    },
  },
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApplicationTextData {
  pub application_name: String,
  pub window_title: String,
  pub text_content: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
  pub window_title: String,
  pub process_id: u32,
  pub application_name: String,
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

      // Create cache request for efficient property access
      let cache_request = automation
        .CreateCacheRequest()
        .map_err(|e| format!("Failed to create cache request: {:?}", e))?;
      
      // Cache the properties we need
      cache_request
        .AddProperty(UIA_NamePropertyId)
        .map_err(|e| format!("Failed to add Name property to cache: {:?}", e))?;
      
      cache_request
        .AddProperty(UIA_ControlTypePropertyId)
        .map_err(|e| format!("Failed to add ControlType property to cache: {:?}", e))?;
      
      cache_request
        .AddProperty(UIA_IsOffscreenPropertyId)
        .map_err(|e| format!("Failed to add IsOffscreen property to cache: {:?}", e))?;
      
      // Set tree scope to subtree for efficient traversal
      cache_request
        .SetTreeScope(TreeScope_Subtree)
        .map_err(|e| format!("Failed to set tree scope: {:?}", e))?;

      println!("Cache request created");

      // Create condition for visible text elements only
      let visible_condition = create_visible_text_condition(&automation)?;
      
      println!("Condition created, finding all visible text elements...");

      // Find ALL visible text elements directly from root - this is much more efficient
      let text_elements = root
        .FindAllBuildCache(TreeScope_Subtree, &visible_condition, &cache_request)
        .map_err(|e| format!("Failed to find text elements: {:?}", e))?;

      let element_count = text_elements
        .Length()
        .map_err(|e| format!("Failed to get element count: {:?}", e))?;

      println!("Found {} text elements", element_count);

      // Group text elements by their parent window/application
      let mut app_map: HashMap<String, ApplicationTextData> = HashMap::new();

      for i in 0..element_count {
        let element = text_elements
          .GetElement(i)
          .map_err(|e| format!("Failed to get element {}: {:?}", i, e))?;

        // Get the text content
        let text = if let Ok(name_bstr) = element.CachedName() {
          let text = name_bstr.to_string();
          if text.trim().is_empty() || text.len() <= 1 {
            continue; // Skip empty or single-character text
          }
          text
        } else {
          continue; // Skip elements without text
        };

        // Find the parent window for this text element
        let (app_name, window_title) = get_parent_window_info(&element)?;
        
        // Create a unique key for this application window
        let app_key = format!("{}::{}", app_name, window_title);
        
        // Add text to the appropriate application group
        app_map.entry(app_key).or_insert_with(|| ApplicationTextData {
          application_name: app_name.clone(),
          window_title: window_title.clone(),
          text_content: Vec::new(),
        }).text_content.push(text);
      }

      // Convert HashMap to Vec
      let applications_data: Vec<ApplicationTextData> = app_map.into_values().collect();

      println!("Grouped into {} applications", applications_data.len());
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
    // Create cache request for efficient property access
    let cache_request = automation
      .CreateCacheRequest()
      .map_err(|e| format!("Failed to create cache request: {:?}", e))?;
    
    // Cache the properties we need
    cache_request
      .AddProperty(UIA_NamePropertyId)
      .map_err(|e| format!("Failed to add Name property to cache: {:?}", e))?;
    
    cache_request
      .AddProperty(UIA_ControlTypePropertyId)
      .map_err(|e| format!("Failed to add ControlType property to cache: {:?}", e))?;
    
    cache_request
      .AddProperty(UIA_IsOffscreenPropertyId)
      .map_err(|e| format!("Failed to add IsOffscreen property to cache: {:?}", e))?;
    
    // Set tree scope to subtree for efficient traversal
    cache_request
      .SetTreeScope(TreeScope_Subtree)
      .map_err(|e| format!("Failed to set tree scope: {:?}", e))?;

    // Create condition for visible text elements only
    let visible_condition = create_visible_text_condition(automation)?;

    // Use FindAllBuildCache for efficient querying
    let elements = window
      .FindAllBuildCache(TreeScope_Subtree, &visible_condition, &cache_request)
      .map_err(|e| format!("Failed to find text elements with cache: {:?}", e))?;

    let count = elements
      .Length()
      .map_err(|e| format!("Failed to get element count: {:?}", e))?;

    let mut text_content = Vec::new();

    for i in 0..count {
      let element = elements
        .GetElement(i)
        .map_err(|e| format!("Failed to get element {}: {:?}", i, e))?;

      // Get cached name property (more efficient than CurrentName)
      if let Ok(name_bstr) = element.CachedName() {
        let text = name_bstr.to_string();
        if !text.trim().is_empty() && text.len() > 1 {
          text_content.push(text);
        }
      }

      // Also try to get value for edit controls
      if let Ok(value_pattern) = element.GetCachedPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId) {
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

fn create_visible_text_condition(
  automation: &IUIAutomation,
) -> Result<windows::Win32::UI::Accessibility::IUIAutomationCondition, String> {
  unsafe {
    // Create condition for text control types
    let text_type_variant = VARIANT::from(UIA_TextControlTypeId.0 as i32);
    let text_condition = automation
      .CreatePropertyCondition(UIA_ControlTypePropertyId, &text_type_variant)
      .map_err(|e| format!("Failed to create text condition: {:?}", e))?;

    // Create condition for edit control types
    let edit_type_variant = VARIANT::from(UIA_EditControlTypeId.0 as i32);
    let edit_condition = automation
      .CreatePropertyCondition(UIA_ControlTypePropertyId, &edit_type_variant)
      .map_err(|e| format!("Failed to create edit condition: {:?}", e))?;

    // Create condition for document control types
    let document_type_variant = VARIANT::from(UIA_DocumentControlTypeId.0 as i32);
    let document_condition = automation
      .CreatePropertyCondition(UIA_ControlTypePropertyId, &document_type_variant)
      .map_err(|e| format!("Failed to create document condition: {:?}", e))?;

    // Create condition for visible elements (not offscreen)
    let visible_variant = VARIANT::from(false); // IsOffscreen = false means visible
    let visible_condition = automation
      .CreatePropertyCondition(UIA_IsOffscreenPropertyId, &visible_variant)
      .map_err(|e| format!("Failed to create visible condition: {:?}", e))?;

    // Combine text-related conditions with OR
    let text_or_edit_condition = automation
      .CreateOrCondition(&text_condition, &edit_condition)
      .map_err(|e| format!("Failed to create text OR edit condition: {:?}", e))?;

    let all_text_condition = automation
      .CreateOrCondition(&text_or_edit_condition, &document_condition)
      .map_err(|e| format!("Failed to create all text condition: {:?}", e))?;

    // Combine with visible condition using AND
    let final_condition = automation
      .CreateAndCondition(&all_text_condition, &visible_condition)
      .map_err(|e| format!("Failed to create final condition: {:?}", e))?;

    Ok(final_condition)
  }
}

fn get_parent_window_info(
  element: &windows::Win32::UI::Accessibility::IUIAutomationElement,
) -> Result<(String, String), String> {
  unsafe {
    let mut current_element = element.clone();
    
    // Walk up the tree to find the parent window
    loop {
      // Try to get the parent
      match current_element.GetCachedParent() {
        Ok(parent) => {
          // Check if this parent is a window
          if let Ok(control_type_variant) = parent.CachedControlType() {
            let control_type = control_type_variant.0 as i32;
            if control_type == UIA_WindowControlTypeId.0 as i32 {
              // This is a window, get its info
              let window_title = if let Ok(name_bstr) = parent.CachedName() {
                let title = name_bstr.to_string();
                if title.trim().is_empty() {
                  "Untitled Window".to_string()
                } else {
                  title
                }
              } else {
                "Unknown Window".to_string()
              };
              
              let app_name = if let Ok(pid) = parent.CurrentProcessId() {
                format!("PID_{}", pid)
              } else {
                "Unknown App".to_string()
              };
              
              return Ok((app_name, window_title));
            }
          }
          current_element = parent;
        }
        Err(_) => {
          // No more parents, use fallback
          return Ok(("Unknown App".to_string(), "Unknown Window".to_string()));
        }
      }
    }
  }
}

#[tauri::command]
pub fn get_all_visible_windows() -> Result<Vec<WindowInfo>, String> {
  println!("get_all_visible_windows called");
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

      // Find all top-level windows (just metadata, not content)
      let windows = root
        .FindAll(TreeScope_Children, &window_condition)
        .map_err(|e| format!("Failed to find windows: {:?}", e))?;
      
      println!("Found windows");

      let window_count = windows
        .Length()
        .map_err(|e| format!("Failed to get window count: {:?}", e))?;

      println!("Found {} windows", window_count);

      let mut window_list = Vec::new();

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

        // Get process ID and application name
        let (process_id, application_name) = match window.CurrentProcessId() {
          Ok(pid) => (pid, format!("PID_{}", pid)),
          Err(_) => (0, "Unknown".to_string()),
        };

        window_list.push(WindowInfo {
          window_title,
          process_id: process_id.try_into().unwrap(),
          application_name,
        });
      }

      println!("Collected {} visible windows", window_list.len());
      Ok(window_list)
    })();

    CoUninitialize();
    result
  }
}
