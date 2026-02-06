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
//! Implemented using embedded RustPython VM in an isolated subprocess.
//! The subprocess provides complete isolation - stack overflows, infinite loops,
//! and other crashes cannot affect the main application.

use super::ToolCall;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Timeout for code execution in seconds
const EXECUTION_TIMEOUT_SECS: u64 = 20;

/// Test command to execute Python code directly from frontend
#[tauri::command]
pub async fn test_python_execution(code: String) -> Result<Value, String> {
    log::info!("[code_execution] Testing Python code from frontend");
    run_python_in_subprocess(&code).await
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
    run_python_in_subprocess(&code).await
}

/// Runs Python code in a completely isolated subprocess.
///
/// This provides complete process isolation - if the Python code causes:
/// - Stack overflow
/// - Infinite loop
/// - Segmentation fault
/// - Any other crash
///
/// Only the subprocess is affected. The main application continues running.
async fn run_python_in_subprocess(code: &str) -> Result<Value, String> {
    let code = code.to_string();
    
    // Run in a blocking task since we're doing process I/O
    tokio::task::spawn_blocking(move || {
        run_python_subprocess_sync(&code)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
}

/// Synchronous subprocess execution with timeout
fn run_python_subprocess_sync(code: &str) -> Result<Value, String> {
    // Get the path to the current executable
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get current executable path: {}", e))?;

    log::info!("[code_execution] Spawning isolated subprocess: {:?}", exe_path);

    // Spawn the subprocess with --exec-python flag
    let mut cmd = Command::new(&exe_path);
    cmd.arg("--exec-python")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    // On Windows, create a new process group so we can terminate it cleanly
    #[cfg(windows)]
    cmd.creation_flags(0x00000200); // CREATE_NEW_PROCESS_GROUP
    
    let mut child = cmd.spawn()
        .map_err(|e| format!("Failed to spawn subprocess: {}", e))?;

    // Send the code to the subprocess via stdin
    {
        let stdin = child.stdin.as_mut()
            .ok_or_else(|| "Failed to open subprocess stdin".to_string())?;
        
        // Write code length first (as a simple protocol)
        let code_bytes = code.as_bytes();
        writeln!(stdin, "{}", code_bytes.len())
            .map_err(|e| format!("Failed to write code length: {}", e))?;
        stdin.write_all(code_bytes)
            .map_err(|e| format!("Failed to write code: {}", e))?;
        stdin.flush()
            .map_err(|e| format!("Failed to flush stdin: {}", e))?;
    }
    // Drop stdin to signal EOF
    drop(child.stdin.take());

    // Wait for the subprocess with timeout
    let timeout = Duration::from_secs(EXECUTION_TIMEOUT_SECS);
    let start = std::time::Instant::now();
    
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished - read all output
                let stdout = child.stdout.take()
                    .map(|mut s| {
                        let mut output = String::new();
                        let _ = std::io::Read::read_to_string(&mut s, &mut output);
                        output
                    })
                    .unwrap_or_default();

                let stderr = child.stderr.take()
                    .map(|mut s| {
                        let mut output = String::new();
                        let _ = std::io::Read::read_to_string(&mut s, &mut output);
                        output
                    })
                    .unwrap_or_default();

                if !status.success() {
                    log::warn!("[code_execution] Subprocess exited with status: {:?}. Stderr: {}", status, stderr);
                    // Check if it was killed/crashed
                    if !status.success() && stdout.trim().is_empty() {
                        return Ok(serde_json::json!({
                            "success": false,
                            "stdout": "",
                            "stderr": format!("Execution crashed or was terminated (exit code: {:?})\n{}", status.code(), stderr)
                        }));
                    }
                }

                // Parse the JSON result from stdout
                let stdout_trimmed = stdout.trim();
                if stdout_trimmed.is_empty() {
                    return Ok(serde_json::json!({
                        "success": false,
                        "stdout": "",
                        "stderr": if stderr.is_empty() { 
                            "Subprocess produced no output".to_string() 
                        } else { 
                            stderr 
                        }
                    }));
                }

                return serde_json::from_str(stdout_trimmed)
                    .map_err(|e| format!("Failed to parse subprocess output: {} (output was: {})", e, stdout_trimmed));
            }
            Ok(None) => {
                // Process still running, check timeout
                if start.elapsed() > timeout {
                    log::warn!("[code_execution] Subprocess timed out after {} seconds, killing", EXECUTION_TIMEOUT_SECS);
                    let _ = child.kill();
                    let _ = child.wait(); // Reap the zombie process
                    return Ok(serde_json::json!({
                        "success": false,
                        "stdout": "",
                        "stderr": format!("Execution timed out after {} seconds", EXECUTION_TIMEOUT_SECS)
                    }));
                }
                // Sleep a bit before checking again
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return Err(format!("Error waiting for subprocess: {}", e));
            }
        }
    }
}

// ============================================================================
// Isolated Python Executor (runs in subprocess)
// ============================================================================

/// Entry point for the isolated Python executor subprocess.
///
/// This function is called when the application is started with `--exec-python`.
/// It reads Python code from stdin, executes it using RustPython, and writes
/// the result as JSON to stdout.
///
/// This runs in a completely separate process, so any crashes (stack overflow,
/// infinite loops caught by timeout in parent, etc.) only affect this subprocess.
pub fn run_isolated_python_executor() {
    // Read code from stdin
    let code = match read_code_from_stdin() {
        Ok(code) => code,
        Err(e) => {
            let result = serde_json::json!({
                "success": false,
                "stdout": "",
                "stderr": format!("Failed to read code: {}", e)
            });
            println!("{}", result);
            return;
        }
    };

    // Execute the Python code
    let result = run_python_script_internal(&code);
    
    // Output result as JSON
    println!("{}", result);
}

/// Reads Python code from stdin using our simple protocol
fn read_code_from_stdin() -> Result<String, String> {
    use std::io::{BufRead, Read};
    
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();
    
    // Read the length line
    let mut length_line = String::new();
    reader.read_line(&mut length_line)
        .map_err(|e| format!("Failed to read length: {}", e))?;
    
    let length: usize = length_line.trim().parse()
        .map_err(|e| format!("Failed to parse length '{}': {}", length_line.trim(), e))?;
    
    // Read exactly that many bytes
    let mut code = vec![0u8; length];
    reader.read_exact(&mut code)
        .map_err(|e| format!("Failed to read code: {}", e))?;
    
    String::from_utf8(code)
        .map_err(|e| format!("Invalid UTF-8 in code: {}", e))
}

/// Internal Python execution using RustPython VM
fn run_python_script_internal(source: &str) -> Value {
    use rustpython_vm::{
        compiler::Mode,
        Interpreter,
        Settings,
        builtins::PyList,
    };

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
