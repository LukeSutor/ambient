use crate::types::AppState;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::result::Result;
use tauri::{AppHandle, Manager};
use tokio::task;
use windows::{
  core::*,
  Win32::{
    Foundation::CloseHandle,
    System::{
      Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED,
      },
      Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
      },
      ProcessStatus::GetModuleBaseNameW,
      Threading::{OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
      Variant::VARIANT,
    },
    UI::Accessibility::{
      CUIAutomation, IUIAutomation, IUIAutomationValuePattern,
      TreeScope_Descendants, TreeScope_Subtree, UIA_ControlTypePropertyId,
      UIA_DocumentControlTypeId, UIA_EditControlTypeId, UIA_IsOffscreenPropertyId,
      UIA_NamePropertyId, UIA_ProcessIdPropertyId, UIA_TextControlTypeId, UIA_ValuePatternId,
    },
  },
};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApplicationTextData {
  pub process_id: i32,
  pub process_name: Option<String>,
  pub application_name: Option<String>,
  pub text_content: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowInfo {
  pub window_title: String,
  pub process_id: u32,
  pub application_name: String,
}

// Function to get all child processes (including nested children) of a parent PID
fn get_child_processes(parent_pid: u32) -> Result<HashSet<u32>, String> {
  unsafe {
    let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
      .map_err(|e| format!("Failed to create snapshot: {:?}", e))?;

    let mut process_entry = PROCESSENTRY32W {
      dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
      ..Default::default()
    };

    let mut child_pids = HashSet::new();
    child_pids.insert(parent_pid); // Include the parent itself

    if Process32FirstW(snapshot, &mut process_entry).is_ok() {
      loop {
        if process_entry.th32ParentProcessID == parent_pid {
          child_pids.insert(process_entry.th32ProcessID);

          // Recursively get children of children
          if let Ok(grandchildren) = get_child_processes(process_entry.th32ProcessID) {
            child_pids.extend(grandchildren);
          }
        }

        if Process32NextW(snapshot, &mut process_entry).is_err() {
          break;
        }
      }
    }

    let _ = CloseHandle(snapshot);
    Ok(child_pids)
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

pub fn get_screen_text_by_application(app_pid: u32) -> Result<Vec<ApplicationTextData>, String> {
  unsafe {
    let hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    if hr.is_err() {
      return Err(format!("CoInitializeEx failed: {:?}", hr));
    }

    let result = (|| {
      // Get all child processes to filter out
      let processes_to_filter = get_child_processes(app_pid).unwrap_or_else(|_| {
        let mut set = HashSet::new();
        set.insert(app_pid); // Fallback to just parent
        set
      });

      let automation: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
        .map_err(|e| format!("Failed to create UIAutomation: {:?}", e))?;

      let root = automation
        .GetRootElement()
        .map_err(|e| format!("Failed to get root element: {:?}", e))?;

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

      cache_request
        .AddProperty(UIA_ProcessIdPropertyId)
        .map_err(|e| format!("Failed to add ProcessId property to cache: {:?}", e))?;

      // Set tree scope to subtree for efficient traversal
      cache_request
        .SetTreeScope(TreeScope_Subtree)
        .map_err(|e| format!("Failed to set tree scope: {:?}", e))?;

      // Create condition for visible text elements only
      let visible_condition = create_visible_text_condition(&automation)?;

      // Find ALL visible text elements directly from root
      let text_elements = root
        .FindAllBuildCache(TreeScope_Subtree, &visible_condition, &cache_request)
        .map_err(|e| format!("Failed to find text elements: {:?}", e))?;

      let element_count = text_elements
        .Length()
        .map_err(|e| format!("Failed to get element count: {:?}", e))?;

      // Group text elements by their PID
      let mut app_map: HashMap<i32, ApplicationTextData> = HashMap::new();

      for i in 0..element_count {
        let element = text_elements
          .GetElement(i)
          .map_err(|e| format!("Failed to get element {}: {:?}", i, e))?;

        // Get the process ID from the cached element
        let process_id = if let Ok(pid) = element.CachedProcessId() {
          pid
        } else {
          continue; // Skip elements without process ID
        };

        // Skip if this matches any process we want to filter (parent + children)
        if processes_to_filter.contains(&(process_id as u32)) {
          continue;
        }

        // Get the text content
        let text = if let Ok(name_bstr) = element.CachedName() {
          let text = name_bstr.to_string();
          if is_junk_text(&text) {
            continue; // Skip junk text using comprehensive filter
          }
          text.trim().to_string()
        } else {
          continue; // Skip elements without text
        };

        // Add text to the appropriate process group
        let app_entry = app_map
          .entry(process_id)
          .or_insert_with(|| ApplicationTextData {
            process_id,
            process_name: None,
            application_name: None,
            text_content: Vec::new(),
          });

        // Only add if not already present (deduplication)
        if !app_entry.text_content.contains(&text) {
          app_entry.text_content.push(text);
        }
      }

      // Convert HashMap to Vec
      let applications_data: Vec<ApplicationTextData> = app_map.into_values().collect();

      Ok(applications_data)
    })();

    CoUninitialize();
    result
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

// Helper function to filter out junk text
fn is_junk_text(text: &str) -> bool {
  let text = text.trim();

  // Filter out empty strings and single characters
  if text.is_empty() || text.len() <= 1 {
    return true;
  }

  // Filter out strings that are just whitespace or special characters
  if text
    .chars()
    .all(|c| c.is_whitespace() || c.is_ascii_punctuation())
  {
    return true;
  }

  // Filter out strings that contain only non-printable characters or replacement characters
  if text
    .chars()
    .all(|c| c.is_control() || c == '\u{FFFD}' || c == '�')
  {
    return true;
  }

  // Filter out strings containing Unicode Private Use Area characters (icon fonts, custom symbols)
  if text.chars().any(|c| {
    let code = c as u32;
    // Private Use Area ranges
    (code >= 0xE000 && code <= 0xF8FF) || // Basic Private Use Area
    (code >= 0xF0000 && code <= 0xFFFFD) || // Supplementary Private Use Area-A
    (code >= 0x100000 && code <= 0x10FFFD) // Supplementary Private Use Area-B
  }) {
    return true;
  }

  // Filter out strings that are mostly Unicode replacement characters (emoji fallbacks)
  let replacement_count = text
    .chars()
    .filter(|&c| c == '\u{FFFD}' || c == '�')
    .count();
  if replacement_count > 0 && replacement_count >= text.chars().count() / 2 {
    return true;
  }

  // Filter out strings that are just whitespace mixed with replacement characters
  if text
    .chars()
    .all(|c| c.is_whitespace() || c == '\u{FFFD}' || c == '�' || c.is_control())
  {
    return true;
  }

  // Filter out very short strings that are likely UI artifacts
  if text.len() <= 3
    && text
      .chars()
      .all(|c| c.is_ascii_digit() || c == '.' || c == ',' || c == '$')
  {
    return true;
  }

  // Filter out strings that are just numbers or basic UI text
  if text
    .chars()
    .all(|c| c.is_ascii_digit() || c.is_whitespace())
  {
    return true;
  }

  // Filter out strings that are only special Unicode characters (box drawing, etc.)
  if text.chars().all(|c| {
    let code = c as u32;
    // Box drawing characters, geometric shapes, symbols, etc.
    (code >= 0x2500 && code <= 0x257F) || // Box drawing
    (code >= 0x2580 && code <= 0x259F) || // Block elements
    (code >= 0x25A0 && code <= 0x25FF) || // Geometric shapes
    (code >= 0x2600 && code <= 0x26FF) || // Miscellaneous symbols
    (code >= 0x2700 && code <= 0x27BF) || // Dingbats
    c.is_whitespace()
  }) {
    return true;
  }

  // Filter out common UI elements that aren't useful
  let lower_text = text.to_lowercase();
  if lower_text == "ok"
    || lower_text == "cancel"
    || lower_text == "close"
    || lower_text == "minimize"
    || lower_text == "maximize"
    || lower_text == "restore"
    || lower_text == "help"
    || lower_text == "file"
    || lower_text == "edit"
    || lower_text == "view"
    || lower_text == "new"
    || lower_text == "save"
  {
    return true;
  }

  false
}

// Function to format application data as markdown
pub fn format_as_markdown(applications: Vec<ApplicationTextData>) -> String {
  let mut markdown = String::new();
  markdown.push_str("# Screen Text by Application\n\n");

  for app in applications {
    // Skip apps with no meaningful content
    if app.text_content.is_empty() {
      continue;
    }

    // Get the app name from the pid and map it to a common name
    let (_process_name, app_name) = if app.process_name.is_some() && app.application_name.is_some()
    {
      (
        app.process_name.clone().unwrap(),
        app.application_name.clone().unwrap(),
      )
    } else {
      let process_name = get_process_name(app.process_id as u32);
      let app_name = map_process_name_to_app_name(&process_name);
      (process_name, app_name)
    };
    markdown.push_str(&format!("## {} (PID: {})\n\n", app_name, app.process_id));

    // Text is already cleaned and deduplicated in get_screen_text_by_application
    for text in &app.text_content {
      markdown.push_str(&format!("{}\n", text));
    }

    markdown.push_str("\n");
  }

  if markdown == "# Screen Text by Application\n\n" {
    markdown.push_str("*No meaningful text content found.*\n");
  }

  markdown
}

pub async fn get_screen_text(app_handle: AppHandle) -> Result<Vec<ApplicationTextData>, String> {
  // Get app PID from app state
  let state = app_handle.state::<AppState>();
  let app_pid = state.pid;

  // Get the screen text in another thread
  let result = task::spawn_blocking(move || get_screen_text_by_application(app_pid)).await;

  match result {
    Ok(applications) => {
      // Get the app name from the PID for each application
      let mut applications_data = applications?;
      for app in &mut applications_data {
        app.process_name = Some(get_process_name(app.process_id as u32));
        app.application_name = Some(map_process_name_to_app_name(
          &app.process_name.as_ref().unwrap(),
        ));
      }
      Ok(applications_data)
    }
    Err(e) => Err(format!("Task execution failed: {:?}", e)),
  }
}

// Parent function that gets screen text and formats it as markdown
#[tauri::command]
pub async fn get_screen_text_formatted(app_handle: AppHandle) -> Result<String, String> {
  // Get app PID from app state
  let state = app_handle.state::<AppState>();
  let app_pid = state.pid;

  // Get the screen text in another thread
  let result = task::spawn_blocking(move || get_screen_text_by_application(app_pid)).await;

  match result {
    Ok(Ok(applications)) => {
      let markdown = format_as_markdown(applications);
      Ok(markdown)
    }
    Ok(Err(e)) => Err(e),
    Err(e) => Err(format!("Task execution failed: {:?}", e)),
  }
}

// Helper function to get process name from PID
fn get_process_name(pid: u32) -> String {
  unsafe {
    let process_handle = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid);

    if let Ok(handle) = process_handle {
      let mut module_name = [0u16; 260]; // MAX_PATH
      let result = GetModuleBaseNameW(handle, None, &mut module_name);

      let _ = CloseHandle(handle);

      if result > 0 {
        // Convert wide string to String
        let name = String::from_utf16_lossy(&module_name[..result as usize]);
        return name;
      }
    }

    format!("Unknown app name") // Fallback to PID if we can't get the name
  }
}

// Helper function to map process names to user-friendly app names
fn map_process_name_to_app_name(process_name: &str) -> String {
  match process_name.to_lowercase().as_str() {
    // Code editors and IDEs
    "code.exe" => "Visual Studio Code".to_string(),
    "devenv.exe" => "Visual Studio".to_string(),
    "notepad++.exe" => "Notepad++".to_string(),
    "sublime_text.exe" => "Sublime Text".to_string(),
    "atom.exe" => "Atom".to_string(),
    "webstorm64.exe" | "webstorm.exe" => "WebStorm".to_string(),
    "idea64.exe" | "idea.exe" => "IntelliJ IDEA".to_string(),
    "pycharm64.exe" | "pycharm.exe" => "PyCharm".to_string(),
    "phpstorm64.exe" | "phpstorm.exe" => "PhpStorm".to_string(),
    "clion64.exe" | "clion.exe" => "CLion".to_string(),
    "rider64.exe" | "rider.exe" => "JetBrains Rider".to_string(),
    "vim.exe" | "gvim.exe" => "Vim".to_string(),
    "emacs.exe" => "Emacs".to_string(),
    "notepad.exe" => "Notepad".to_string(),

    // Web browsers
    "chrome.exe" => "Google Chrome".to_string(),
    "firefox.exe" => "Mozilla Firefox".to_string(),
    "msedge.exe" => "Microsoft Edge".to_string(),
    "brave.exe" => "Brave Browser".to_string(),
    "opera.exe" => "Opera".to_string(),
    "safari.exe" => "Safari".to_string(),
    "vivaldi.exe" => "Vivaldi".to_string(),
    "iexplore.exe" => "Internet Explorer".to_string(),

    // Communication apps
    "discord.exe" => "Discord".to_string(),
    "slack.exe" => "Slack".to_string(),
    "teams.exe" | "ms-teams.exe" => "Microsoft Teams".to_string(),
    "zoom.exe" => "Zoom".to_string(),
    "skype.exe" => "Skype".to_string(),
    "whatsapp.exe" => "WhatsApp".to_string(),
    "telegram.exe" => "Telegram".to_string(),
    "signal.exe" => "Signal".to_string(),

    // Office applications
    "winword.exe" => "Microsoft Word".to_string(),
    "excel.exe" => "Microsoft Excel".to_string(),
    "powerpnt.exe" => "Microsoft PowerPoint".to_string(),
    "outlook.exe" => "Microsoft Outlook".to_string(),
    "onenote.exe" => "Microsoft OneNote".to_string(),
    "visio.exe" => "Microsoft Visio".to_string(),
    "project.exe" => "Microsoft Project".to_string(),
    "access.exe" => "Microsoft Access".to_string(),

    // Media and entertainment
    "spotify.exe" => "Spotify".to_string(),
    "vlc.exe" => "VLC Media Player".to_string(),
    "steam.exe" => "Steam".to_string(),
    "epicgameslauncher.exe" => "Epic Games Launcher".to_string(),
    "netflix.exe" => "Netflix".to_string(),
    "youtube.exe" => "YouTube".to_string(),
    "itunes.exe" => "iTunes".to_string(),
    "audacity.exe" => "Audacity".to_string(),

    // System and utilities
    "explorer.exe" => "Windows Explorer".to_string(),
    "cmd.exe" => "Command Prompt".to_string(),
    "powershell.exe" => "PowerShell".to_string(),
    "winrar.exe" => "WinRAR".to_string(),
    "7zfm.exe" => "7-Zip".to_string(),
    "taskmgr.exe" => "Task Manager".to_string(),
    "regedit.exe" => "Registry Editor".to_string(),
    "mmc.exe" => "Microsoft Management Console".to_string(),
    "control.exe" => "Control Panel".to_string(),
    "calc.exe" => "Calculator".to_string(),
    "mspaint.exe" => "Paint".to_string(),
    "snip.exe" | "snippingtool.exe" => "Snipping Tool".to_string(),

    // Development tools
    "git.exe" => "Git".to_string(),
    "node.exe" => "Node.js".to_string(),
    "python.exe" => "Python".to_string(),
    "java.exe" | "javaw.exe" => "Java".to_string(),
    "docker.exe" => "Docker".to_string(),
    "postman.exe" => "Postman".to_string(),
    "fiddler.exe" => "Fiddler".to_string(),
    "wireshark.exe" => "Wireshark".to_string(),

    // Graphics and design
    "photoshop.exe" => "Adobe Photoshop".to_string(),
    "illustrator.exe" => "Adobe Illustrator".to_string(),
    "indesign.exe" => "Adobe InDesign".to_string(),
    "aftereffects.exe" => "Adobe After Effects".to_string(),
    "premiere.exe" => "Adobe Premiere Pro".to_string(),
    "figma.exe" => "Figma".to_string(),
    "blender.exe" => "Blender".to_string(),
    "gimp.exe" => "GIMP".to_string(),

    // Security
    "windefend.exe" => "Windows Defender".to_string(),
    "mbam.exe" => "Malwarebytes".to_string(),
    "avast.exe" => "Avast Antivirus".to_string(),
    "avg.exe" => "AVG Antivirus".to_string(),
    "norton.exe" => "Norton Antivirus".to_string(),

    // Database tools
    "ssms.exe" => "SQL Server Management Studio".to_string(),
    "mysql.exe" => "MySQL".to_string(),
    "postgres.exe" => "PostgreSQL".to_string(),
    "mongodb.exe" => "MongoDB".to_string(),

    // Virtual machines
    "vmware.exe" => "VMware".to_string(),
    "virtualbox.exe" => "VirtualBox".to_string(),

    // If no mapping found, return the original process name
    _ => {
      // Remove .exe extension if present for cleaner display
      if process_name.to_lowercase().ends_with(".exe") {
        process_name[..process_name.len() - 4].to_string()
      } else {
        process_name.to_string()
      }
    }
  }
}
