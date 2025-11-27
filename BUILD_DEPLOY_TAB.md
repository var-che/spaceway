# Build & Deploy Tab - Dashboard Feature

## Overview

Added a new **Build & Deploy** tab to the dashboard frontend that allows you to compile and manage the Spaceway project without using the terminal directly.

## Features

### Available Build Tasks

1. **📦 Compile Core Library**

   - Command: `cargo build --package spaceway-core`
   - Builds the spaceway-core Rust library
   - Run this first if you made changes to core

2. **🔧 Compile Dashboard Backend**

   - Command: `cargo build --package dashboard-backend`
   - Builds the dashboard backend server
   - Run after making core changes

3. **🚀 Start Dashboard Backend**

   - Command: `cargo +nightly run`
   - Starts the backend server on port 3030
   - Runs in background mode

4. **🛑 Stop Dashboard Backend**

   - Command: `pkill -f dashboard-backend`
   - Stops the running backend server

5. **🔄 Restart Dashboard Backend**

   - Command: `pkill -f dashboard-backend && cargo +nightly run`
   - Stops and restarts the backend
   - Use this to apply code changes

6. **🧹 Clean Build Artifacts**

   - Command: `cargo clean`
   - Removes all compiled artifacts
   - Use when you need a fresh build

7. **✅ Check Core (Fast)**

   - Command: `cargo check --package spaceway-core`
   - Quick syntax check without full compilation
   - Fast validation during development

8. **🧪 Run Core Tests**
   - Command: `cargo test --package spaceway-core`
   - Runs all tests in spaceway-core
   - Verify your changes work correctly

## How to Use

### Accessing the Build Tab

1. Open the dashboard at `http://localhost:5173`
2. Click the **🔧 Build & Deploy** tab in the header (next to "📊 Dashboard")

### Running a Task

1. Click any task button (e.g., "📦 Compile Core Library")
2. The task will execute and show "⏳ Running..." status
3. When complete, click "▶ Show Output" to see the command output
4. Check the exit code (green = success, red = error)

### Typical Workflow

**Making Changes to Core:**

```
1. Edit code in core/src/
2. Click "✅ Check Core (Fast)" - verify syntax
3. Click "📦 Compile Core Library" - build changes
4. Click "🧪 Run Core Tests" - verify tests pass
5. Click "🔄 Restart Dashboard Backend" - apply changes
6. Switch to "📊 Dashboard" tab - test your changes
```

**Starting from Scratch:**

```
1. Click "📦 Compile Core Library"
2. Click "🔧 Compile Dashboard Backend"
3. Click "🚀 Start Dashboard Backend"
4. Switch to "📊 Dashboard" tab
```

## Technical Details

### Backend API

- **Endpoint:** `POST http://localhost:3030/api/build`
- **Request:**
  ```json
  {
    "command": "cargo build",
    "working_dir": "/home/vlada/Documents/projects/spaceway",
    "is_background": false
  }
  ```
- **Response:**
  ```json
  {
    "success": true,
    "message": "Command completed successfully",
    "output": "Compiling spaceway-core...",
    "exit_code": 0
  }
  ```

### Background vs. Foreground Tasks

- **Foreground Tasks:** Wait for completion, return output

  - Compile commands
  - Check/test commands
  - Clean command

- **Background Tasks:** Spawn and return immediately
  - Start backend
  - Restart backend (starts new process in background)

### Files Modified

1. **Frontend:**

   - `dashboard-frontend/src/App.tsx` - Added tab system
   - `dashboard-frontend/src/App.css` - Tab styling
   - `dashboard-frontend/src/components/BuildPanel.tsx` - New component
   - `dashboard-frontend/src/components/BuildPanel.css` - Styling

2. **Backend:**
   - `dashboard-backend/src/main.rs` - Added `/api/build` endpoint

## Benefits

✅ **No Terminal Required** - Execute all build commands from the UI
✅ **Visual Feedback** - See command output and exit codes
✅ **Quick Reference** - Task descriptions help you understand what each does
✅ **Integrated Workflow** - Switch between building and testing without leaving the dashboard
✅ **Error Visibility** - See compilation errors directly in the UI

## Notes

- The frontend will auto-reload when you make changes (Vite hot reload)
- Background tasks won't show output immediately (they run in the background)
- Use "Stop Backend" before manually restarting to avoid port conflicts
- The dashboard must be connected to the backend to use build commands
