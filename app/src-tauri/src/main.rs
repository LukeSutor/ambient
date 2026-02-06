// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Check if we're running in isolated Python execution mode
    // This mode runs Python code in a completely separate process
    // to protect the main application from crashes (stack overflow, etc.)
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--exec-python" {
        tauri_nextjs_template_lib::skills::builtin::code_execution::run_isolated_python_executor();
        return;
    }

    tauri_nextjs_template_lib::run()
}
