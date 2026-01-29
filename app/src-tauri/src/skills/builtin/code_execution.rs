//! Code execution skill implementation.
//!
//! This skill provides sandboxed code execution capabilities.
//!
//! # Tools
//!
//! - `execute_code`: Execute code in a safe environment
//!
//! # Status
//!
//! Implemented using embedded RustPython VM.

use super::ToolCall;
use serde_json::Value;
use rustpython_vm::{
    compiler::Mode,
    Interpreter,
    Settings,
    builtins::PyList,
};

/// Test command to execute Python code directly from frontend
#[tauri::command]
pub async fn test_python_execution(code: String) -> Result<Value, String> {
    log::info!("[code_execution] Testing Python code from frontend");
    let result = tokio::task::spawn_blocking(move || {
        run_python_script(&code)
    })
    .await
    .map_err(|e| format!("Execution task failed: {}", e))?;

    Ok(result)
}

/// Execute a code execution tool.
pub async fn execute(
    _app_handle: &tauri::AppHandle,
    call: &ToolCall,
) -> Result<Value, String> {
    match call.tool_name.as_str() {
        "execute_code" => execute_code(call).await,
        _ => Err(format!("Unknown tool: {}", call.tool_name)),
    }
}

/// Execute code in a safe environment.
async fn execute_code(call: &ToolCall) -> Result<Value, String> {
    let code = call
        .arguments
        .get("code")
        .and_then(|c| c.as_str())
        .ok_or_else(|| "Missing 'code' argument".to_string())?
        .to_string();

    log::info!("[code_execution] Executing Python code");

    // Run in blocking thread as VM is CPU-bound and synchronous
    let result = tokio::task::spawn_blocking(move || {
        run_python_script(&code)
    })
    .await
    .map_err(|e| format!("Execution task failed: {}", e))?;

    Ok(result)
}

fn run_python_script(source: &str) -> Value {
    // Use default settings but without stdlib for sandboxing
    // This ensures no access to os, io, socket, etc.
    let settings = Settings::default();
    let interpreter = Interpreter::without_stdlib(settings);

    interpreter.enter(|vm| {
        let scope = vm.new_scope_with_builtins();

        // Inject output capturing prelude
        // We define a global _OUT list and shadow print() to append to it
        let prelude = r#"
_OUT = []
def print(*args, sep=' ', end='\n', file=None, flush=False):
    try:
        s = sep.join(map(str, args)) + end
        _OUT.append(s)
    except:
        pass
"#;

        if let Ok(code) = vm.compile(prelude, Mode::Exec, "<prelude>".to_owned()) {
             let _ = vm.run_code_obj(code, scope.clone());
        }

        // Run user code
        let code_result = vm
            .compile(source, Mode::Exec, "<user_code>".to_owned())
            .map_err(|err| {
                let mut msg = String::new();
                vm.write_exception(&mut msg, &vm.new_syntax_error(&err, None)).ok();
                msg
            })
            .and_then(|code_obj| {
                vm.run_code_obj(code_obj, scope.clone()).map_err(|exc| {
                    let mut msg = String::new();
                    vm.write_exception(&mut msg, &exc).ok();
                    msg
                })
            });

        // Collect output from _OUT
        let mut stdout = String::new();
        if let Ok(out_list) = scope.globals.get_item("_OUT", vm) {
            if let Some(list) = out_list.payload::<PyList>() {
                let items = list.borrow_vec();
                for item in items.iter() {
                    if let Ok(s) = item.str(vm) {
                        stdout.push_str(s.as_str());
                    }
                }
            }
        }

        match code_result {
            Ok(_) => serde_json::json!({
                "success": true,
                "stdout": stdout,
                "stderr": ""
            }),
            Err(e) => serde_json::json!({
                "success": false,
                "stdout": stdout,
                "stderr": e
            }),
        }
    })
}
