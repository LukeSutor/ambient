# TaskAware: AI-Powered Workflow Tracking for Desktop Employees

**TaskAware** is a privacy-conscious, AI-powered desktop productivity assistant that runs locally on employees' devices. It intelligently monitors screen activity, matches actions to structured tasks, and syncs progress with a centralized dashboard—enabling seamless task tracking without micromanagement or invasive surveillance.

This project is built using **Tauri**, with a **Next.js frontend**, **Rust backend**, and **llama.cpp** for local LLM inference.

---

## 🎯 Project Goal

The goal of TaskAware is to create a system that:

1. **Monitors user activity locally** (e.g. screen text, app usage).
2. **Understands and tracks task completion** using on-device LLMs.
3. **Automatically checks off tasks** when the employee performs them.
4. **Reminds users** when steps are missed.
5. **Reports to managers** in a clean, actionable dashboard.
6. Does all this **securely and ethically**, using **on-device AI** to avoid sending sensitive data to the cloud.

---

## ✅ Features Implemented

- ✅ **Tauri-based Desktop App**: Lightweight, cross-platform, and secure.
- ✅ **Next.js + TailwindCSS + ShadCN UI**: Clean and responsive user interface.
- ✅ **Local LLM Integration (llama.cpp)**: Local text generation and reasoning without needing cloud APIs.
- ✅ **AWS Cognito Auth**: Simple user login/logout system for secure user sessions.
- ✅ **Windows UI Automation Integration (`windows-rs`)**: Extracts on-screen text for parsing and analysis.
- ✅ **Event-driven Rust backend**: Architecture designed for modular event handling.
- ✅ **Utility for model downloading**: Dynamically fetch and install models for local use.

---

## 🛠️ Remaining Work

Below is a checklist of the features and components that still need to be designed and implemented.

### 👁️ Screen Understanding & Text Formatting
- [x] Develop more intelligent screen parsing logic to extract structured information from UI elements.
- [x] Apply heuristics or LLM-driven summarization to remove noise from extracted text.

### 🧠 Task Modeling & Progress Tracking
- [ ] Design a system for representing structured task checklists.
- [ ] Define individual task steps in a machine-readable way that the LLM can reference.
- [ ] Support dynamic task assignment and syncing from a remote database.

### 🤖 LLM-Powered Task Detection
- [ ] Use function-calling LLM workflows to:
  - Parse current screen state
  - Match to known task steps
  - Mark tasks as complete, in progress, or not started
- [ ] Implement a feedback loop where the LLM can nudge the user if steps are skipped.
- [ ] Implement a chat window where users can get clarification about tasks.


### 📊 Manager Dashboard
- [ ] Build a dashboard that:
  - Displays all tasks assigned to each employee
  - Shows task completion status in real time
  - Tracks overall productivity

### 🛡️ Privacy and Control
- [ ] Build a toggle or “pause tracking” feature for employees to control when their activity is monitored.
- [ ] Ensure all monitoring and inference stays strictly local unless explicitly shared by the user.

### 🔄 Integrations & Syncing
- [ ] Sync task progress between local app and remote dashboard/database.
- [ ] Allow new tasks and workflows to be pushed to the client from a centralized interface.

### 🧪 Testing & Optimization
- [ ] Create LLM evals for task tracking.
- [ ] Write unit and integration tests for screen parsing, event handling, and LLM responses.

---

## 🔧 Technologies Used

| Tech | Purpose |
|------|---------|
| **Tauri** | Cross-platform desktop app shell |
| **Rust** | System-level backend logic and screen interaction |
| **Next.js** | UI framework for dashboard and frontend logic |
| **TailwindCSS + ShadCN** | Fast, modern styling and components |
| **AWS Cognito** | User authentication and session management |
| **llama.cpp** | On-device language model inference |
| **windows-rs + UIAutomation API** | Access screen UI elements and capture text on Windows |

---

## 🚀 Vision for the Future

TaskAware is the first step toward an ethical alternative to invasive employee monitoring systems. Instead of screenshots and keyloggers, this system uses AI to **understand work**, **respect privacy**, and **empower both workers and managers**.

We envision future integrations with:
- Voice assistants ("Remind me to finish that report")
- Task auto-prioritization
- Productivity analysis that works *with* the user

---

## 📂 Project Structure

/app/
└── Tauri application with Next.js frontend
/backend/
└── `llama.cpp` code and other utilities
/ml/
└── Model training scripts and machine learning experiments