import { useState } from "react";
import "./BuildPanel.css";

interface BuildTask {
  id: string;
  label: string;
  command: string;
  description: string;
  workingDir: string;
  isBackground?: boolean;
}

const BUILD_TASKS: BuildTask[] = [
  {
    id: "compile-core",
    label: "📦 Compile Core Library",
    command: "cargo build --package spaceway-core",
    description: "Build the spaceway-core Rust library",
    workingDir: "/home/vlada/Documents/projects/spaceway",
    isBackground: false,
  },
  {
    id: "compile-dashboard-backend",
    label: "🔧 Compile Dashboard Backend",
    command: "cargo build --package dashboard-backend",
    description: "Build the dashboard backend server",
    workingDir:
      "/home/vlada/Documents/projects/spaceway/dashbard/dashboard-backend",
    isBackground: false,
  },
  {
    id: "start-dashboard-backend",
    label: "🚀 Start Dashboard Backend",
    command: "cargo +nightly run --bin dashboard-backend",
    description: "Run the dashboard backend server on port 3030",
    workingDir:
      "/home/vlada/Documents/projects/spaceway/dashbard/dashboard-backend",
    isBackground: true,
  },
  {
    id: "stop-dashboard-backend",
    label: "🛑 Stop Dashboard Backend",
    command: "pkill -f dashboard-backend",
    description: "Stop the running dashboard backend server",
    workingDir: "/home/vlada/Documents/projects/spaceway",
    isBackground: false,
  },
  {
    id: "restart-dashboard-backend",
    label: "🔄 Restart Dashboard Backend",
    command:
      "pkill -f dashboard-backend && cd /home/vlada/Documents/projects/spaceway/dashbard/dashboard-backend && cargo +nightly run --bin dashboard-backend",
    description: "Stop and restart the dashboard backend",
    workingDir:
      "/home/vlada/Documents/projects/spaceway/dashbard/dashboard-backend",
    isBackground: true,
  },
  {
    id: "clean-build",
    label: "🧹 Clean Build Artifacts",
    command: "cargo clean",
    description: "Remove all compiled artifacts",
    workingDir: "/home/vlada/Documents/projects/spaceway",
    isBackground: false,
  },
  {
    id: "check-core",
    label: "✅ Check Core (Fast)",
    command: "cargo check --package spaceway-core",
    description: "Quick syntax check without building",
    workingDir: "/home/vlada/Documents/projects/spaceway",
    isBackground: false,
  },
  {
    id: "test-core",
    label: "🧪 Run Core Tests",
    command: "cargo test --package spaceway-core",
    description: "Run all tests in spaceway-core",
    workingDir: "/home/vlada/Documents/projects/spaceway",
    isBackground: false,
  },
];

interface TaskOutput {
  taskId: string;
  output: string;
  isRunning: boolean;
  exitCode?: number;
}

export default function BuildPanel() {
  const [taskOutputs, setTaskOutputs] = useState<Map<string, TaskOutput>>(
    new Map()
  );
  const [expandedTask, setExpandedTask] = useState<string | null>(null);

  const runTask = async (task: BuildTask) => {
    // Update state to show task is running
    setTaskOutputs((prev) => {
      const newMap = new Map(prev);
      newMap.set(task.id, {
        taskId: task.id,
        output: `Running: ${task.command}\n`,
        isRunning: true,
      });
      return newMap;
    });

    // Expand the task output
    setExpandedTask(task.id);

    try {
      // Send request to backend to execute the command
      const response = await fetch("http://localhost:3030/api/build", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          command: task.command,
          working_dir: task.workingDir,
          is_background: task.isBackground || false,
        }),
      });

      const result = await response.json();

      setTaskOutputs((prev) => {
        const newMap = new Map(prev);
        newMap.set(task.id, {
          taskId: task.id,
          output: result.output || result.message || "Task completed",
          isRunning: task.isBackground || false,
          exitCode: result.exit_code,
        });
        return newMap;
      });
    } catch (error) {
      setTaskOutputs((prev) => {
        const newMap = new Map(prev);
        newMap.set(task.id, {
          taskId: task.id,
          output: `Error: ${error}`,
          isRunning: false,
          exitCode: 1,
        });
        return newMap;
      });
    }
  };

  const toggleTaskOutput = (taskId: string) => {
    setExpandedTask(expandedTask === taskId ? null : taskId);
  };

  return (
    <div className="build-panel">
      <div className="build-tasks">
        {BUILD_TASKS.map((task) => {
          const taskOutput = taskOutputs.get(task.id);
          const isRunning = taskOutput?.isRunning || false;
          const isExpanded = expandedTask === task.id;

          return (
            <div key={task.id} className="build-task">
              <div className="task-header">
                <div className="task-info">
                  <button
                    className="task-button"
                    onClick={() => runTask(task)}
                    disabled={isRunning}
                  >
                    {task.label}
                  </button>
                  <span className="task-description">{task.description}</span>
                  {isRunning && (
                    <span className="task-running">⏳ Running...</span>
                  )}
                </div>
                {taskOutput && (
                  <button
                    className="toggle-output-btn"
                    onClick={() => toggleTaskOutput(task.id)}
                  >
                    {isExpanded ? "▼ Hide Output" : "▶ Show Output"}
                  </button>
                )}
              </div>
              {isExpanded && taskOutput && (
                <div className="task-output">
                  <pre>{taskOutput.output}</pre>
                  {taskOutput.exitCode !== undefined && (
                    <div
                      className={`exit-code ${
                        taskOutput.exitCode === 0 ? "success" : "error"
                      }`}
                    >
                      Exit Code: {taskOutput.exitCode}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      <div className="build-info">
        <h3>ℹ️ Quick Reference</h3>
        <ul>
          <li>
            <strong>Compile Core:</strong> Build the core library first if you
            made changes
          </li>
          <li>
            <strong>Compile Backend:</strong> Build the dashboard backend after
            core changes
          </li>
          <li>
            <strong>Start Backend:</strong> Launch the backend server (runs in
            background)
          </li>
          <li>
            <strong>Restart Backend:</strong> Apply code changes by restarting
            the server
          </li>
          <li>
            <strong>Check:</strong> Fast syntax validation without full
            compilation
          </li>
        </ul>
      </div>
    </div>
  );
}
