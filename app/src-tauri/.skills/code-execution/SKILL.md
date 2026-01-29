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
- Only `print()` values are captured. Print any values you want to view.

## Guidelines
- Prefer iteration over recursion
- Prefer efficiency over readability

## Examples

### Calculate Fibonacci
```python
def fib(n):
    a, b = 0, 1
    for i in range(0, n):
        a, b = b, a + b
    return a
print(fib(10))
```

### Text Processing
```python
text = "hello world"
print(text.upper()[::-1])
```
