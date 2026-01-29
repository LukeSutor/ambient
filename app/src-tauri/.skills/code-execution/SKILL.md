---
name: code-execution
description: Execute Python code in a secure, sandboxed environment. Use this for calculations, data processing, logic problems, and text manipulation.
version: "1.0"
requires_auth: false
tools:
  - name: execute_code
    description: Execute Python code and return the output
    parameters:
      code:
        type: string
        description: The Python code to execute
        required: true
---

# Code Execution Skill

This skill allows running Python code in a sandboxed embedded environment.

## Capabilities
- **Pure Python Logic**: Run algorithms, mathematical calculations, and string processing.
- **Sandboxed**: No access to file system or network.
- **State**: Execution is stateless (variables do not persist between calls).

## Limitations
- No standard library I/O (`os`, `sys`, `io`, `socket` are not available).
- No external packages (`numpy`, `pandas`, `requests` are not available).
- Only `print()` and return values are captured.

## Examples

### Calculate Fibonacci
```python
def fib(n):
    if n <= 1: return n
    return fib(n-1) + fib(n-2)
print(fib(10))
```

### Text Processing
```python
text = "hello world"
print(text.upper()[::-1])
```
