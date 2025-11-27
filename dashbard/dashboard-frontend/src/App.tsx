import { useState, useEffect, useRef } from "react";
import "./App.css";
import NetworkGraph from "./components/NetworkGraph";
import ClientPanel from "./components/ClientPanel";
import ActionPanel from "./components/ActionPanel";
import CrdtTimeline from "./components/CrdtTimeline";
import BuildPanel from "./components/BuildPanel";
import TutorialPanel from "./components/TutorialPanel";
import DiscordUI from "./components/DiscordUI";

export interface DashboardState {
  clients: ClientSnapshot[];
  network_graph: NetworkGraph;
  crdt_timeline: CrdtOperation[];
}

export interface ClientSnapshot {
  name: string;
  user_id: string;
  spaces: SpaceInfo[];
  dht_storage: DhtEntry[];
  mls_groups: MlsGroupInfo[];
  connected_peers: string[];
}

export interface SpaceInfo {
  id: string;
  name: string;
  owner: string;
  members: MemberInfo[];
  channels: ChannelInfo[];
  role_count: number;
}

export interface MemberInfo {
  user_id: string;
  role: string;
  permissions: string[];
}

export interface ChannelInfo {
  id: string;
  name: string;
  message_count: number;
}

export interface DhtEntry {
  key: string;
  value_type: string;
  size_bytes: number;
}

export interface MlsGroupInfo {
  space_id: string;
  epoch: number;
  member_count: number;
}

export interface NetworkGraph {
  nodes: NetworkNode[];
  edges: NetworkEdge[];
}

export interface NetworkNode {
  id: string;
  label: string;
  peer_type: string;
}

export interface NetworkEdge {
  from: string;
  to: string;
  edge_type: string;
}

export interface CrdtOperation {
  timestamp: number;
  op_id: string;
  op_type: string;
  author: string;
  space_id: string;
  channel_id?: string;
}

function App() {
  const [state, setState] = useState<DashboardState | null>(null);
  const [connected, setConnected] = useState(false);
  const [activeTab, setActiveTab] = useState<
    "dashboard" | "build" | "tutorial" | "discord"
  >("discord"); // Default to Discord UI
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    // Clear any pending reconnection attempts
    if (reconnectTimeoutRef.current) {
      clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    // Prevent duplicate connections - check if already connected
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      return;
    }

    // Close any existing connection that's in a bad state
    if (wsRef.current && wsRef.current.readyState !== WebSocket.CLOSED) {
      wsRef.current.close();
    }

    // Connect to WebSocket
    const ws = new WebSocket("ws://localhost:3030/ws");
    wsRef.current = ws;

    ws.onopen = () => {
      console.log("🔌 Connected to dashboard backend");
      setConnected(true);
    };

    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setState(data);
    };

    ws.onclose = () => {
      console.log("🔌 Disconnected from dashboard backend");
      setConnected(false);
    };

    ws.onerror = (error) => {
      console.error("❌ WebSocket error:", error);
    };

    return () => {
      // Don't close immediately - React StrictMode might be remounting
      // Instead, schedule a close and cancel it if we remount quickly
      reconnectTimeoutRef.current = setTimeout(() => {
        if (wsRef.current === ws) {
          ws.close();
          wsRef.current = null;
        }
      }, 100);
    };
  }, []);

  if (!connected) {
    return (
      <div className="app loading">
        <h1>🔄 Connecting to Dashboard Backend...</h1>
        <p>
          Make sure the backend is running on <code>localhost:3030</code>
        </p>
      </div>
    );
  }

  if (!state) {
    return (
      <div className="app loading">
        <h1>⏳ Loading state...</h1>
      </div>
    );
  }

  return (
    <div className="app">
      <header>
        <h1>🚀 Discord-Lite Dashboard</h1>
        <div className="tabs">
          <button
            className={`tab ${activeTab === "discord" ? "active" : ""}`}
            onClick={() => setActiveTab("discord")}
          >
            💬 Discord UI
          </button>
          <button
            className={`tab ${activeTab === "dashboard" ? "active" : ""}`}
            onClick={() => setActiveTab("dashboard")}
          >
            📊 Dashboard
          </button>
          <button
            className={`tab ${activeTab === "tutorial" ? "active" : ""}`}
            onClick={() => setActiveTab("tutorial")}
          >
            📚 Tutorial
          </button>
          <button
            className={`tab ${activeTab === "build" ? "active" : ""}`}
            onClick={() => setActiveTab("build")}
          >
            🔧 Build & Deploy
          </button>
        </div>
        <div className="status">
          <span className="connected">● Connected</span>
        </div>
      </header>

      {activeTab === "discord" && <DiscordUI />}

      {activeTab === "dashboard" && (
        <div className="dashboard-grid">
          {/* Action Panel */}
          <div className="panel action-panel">
            <h2>⚡ Actions</h2>
            <ActionPanel />
          </div>

          {/* Network Graph */}
          <div className="panel network-panel">
            <h2>🌐 Network Topology</h2>
            <NetworkGraph
              nodes={state.network_graph.nodes}
              edges={state.network_graph.edges}
            />
          </div>

          {/* Alice */}
          <div className="panel client-panel">
            <h2>👩 Alice</h2>
            <ClientPanel
              client={state.clients.find((c) => c.name === "Alice")!}
            />
          </div>

          {/* Bob */}
          <div className="panel client-panel">
            <h2>👨 Bob</h2>
            <ClientPanel
              client={state.clients.find((c) => c.name === "Bob")!}
            />
          </div>

          {/* Charlie */}
          <div className="panel client-panel">
            <h2>🧑 Charlie</h2>
            <ClientPanel
              client={state.clients.find((c) => c.name === "Charlie")!}
            />
          </div>

          {/* CRDT Timeline */}
          <div className="panel timeline-panel">
            <h2>📊 CRDT Operations Timeline</h2>
            <CrdtTimeline operations={state.crdt_timeline} />
          </div>
        </div>
      )}

      {activeTab === "tutorial" && (
        <div className="tutorial-container">
          <div className="panel tutorial-panel-container">
            <TutorialPanel />
          </div>
        </div>
      )}

      {activeTab === "build" && (
        <div className="build-container">
          <div className="panel build-panel-container">
            <h2>🔧 Build & Deploy Controls</h2>
            <BuildPanel />
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
