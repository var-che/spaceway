# Dashboard Quick Start

## ✅ Project Structure Created

```
dashboard/
├── dashboard-backend/        # Rust + Axum + WebSocket
│   ├── Cargo.toml           # Dependencies configured
│   └── src/main.rs          # 3-client orchestrator + API
│
├── dashboard-frontend/       # React + Vite + TypeScript
│   ├── src/
│   │   ├── App.tsx          # Main dashboard layout
│   │   ├── App.css          # Dark theme styles
│   │   └── components/
│   │       ├── ClientPanel.tsx      # Client state viewer
│   │       ├── ActionPanel.tsx      # Interactive controls
│   │       ├── NetworkGraph.tsx     # SVG network visualization
│   │       └── CrdtTimeline.tsx     # Operation timeline
│   ├── package.json         # Dependencies listed
│   └── vite.config.ts       # Proxy configuration
│
└── README.md                # Full documentation

```

## 🚀 How to Run

### Terminal 1: Start Backend

```bash
cd /home/vlada/Documents/projects/spaceway/dashbard/dashboard-backend
cargo run
```

Expected output:

```
🚀 Starting Dashboard Backend
🎯 Dashboard backend listening on http://127.0.0.1:3030
```

### Terminal 2: Start Frontend (Already Running!)

The frontend is already running on http://localhost:5173

If you need to restart it:

```bash
cd /home/vlada/Documents/projects/spaceway/dashbard/dashboard-frontend
npm run dev
```

## 🎮 Using the Dashboard

1. **Open**: http://localhost:5173 in your browser

2. **Create a Space** (Alice):

   - Client: Alice
   - Action: Create Space
   - Space Name: "Dev Team"
   - Click Execute
   - Watch space appear in Alice's panel

3. **Create a Channel** (Alice):

   - Copy the space ID from Alice's panel (first 8 chars shown)
   - Client: Alice
   - Action: Create Channel
   - Space ID: (paste full UUID)
   - Channel Name: "general"
   - Click Execute

4. **Invite Bob**:

   - Client: Alice
   - Action: Create Invite
   - Space ID: (same as above)
   - Click Execute
   - Copy invite code from response

5. **Bob Joins**:

   - Client: Bob
   - Action: Join Space
   - Invite Code: (paste)
   - Click Execute
   - Watch Bob appear in Alice's space members!

6. **Observe**:
   - Network Graph: Shows connections
   - Client Panels: See each client's storage
   - CRDT Timeline: Operation history

## 📊 What You'll See

### Network Graph

Shows Alice, Bob, Charlie as nodes. Edges appear when they share spaces.

### Client Panels

- User ID
- Spaces joined
- Members & their roles
- Channels in each space
- Permission data

### Action Panel

Interactive controls to trigger any operation on any client.

### CRDT Timeline

Chronological log of all distributed operations.

## 🔍 Debugging

**Backend logs**: Terminal 1 shows all operations

```
📝 Action request: CreateSpace { name: "Dev Team" }
```

**Frontend console**: Browser DevTools → Console

```
🔌 Connected to dashboard backend
```

**WebSocket traffic**: DevTools → Network → WS tab

## 📝 Next Steps

1. **Test permission system**: Try having Bob create a channel (should fail)
2. **Visualize network**: Create space → See network graph update
3. **Track CRDT ops**: Watch timeline as operations flow
4. **Inspect storage**: See what each client stores locally

## 🎯 Key Features to Explore

- **Real-time updates**: State refreshes every 500ms
- **3-client orchestration**: All clients in one process
- **Permission enforcement**: Watch denied operations
- **Network topology**: Visualize P2P connections
- **CRDT causality**: See operation ordering

## 🐛 Troubleshooting

**"Connecting to backend..."**: Start the backend first (Terminal 1)

**Action fails**: Check backend logs for detailed error

**Network graph empty**: Create a space first - nodes appear when clients share spaces

**UUIDs don't work**: Copy the full hex string, not just the displayed 8 chars

## 🎉 Success Criteria

You've successfully set up the dashboard when:

- ✅ Frontend shows "Connected" status
- ✅ Can create a space with Alice
- ✅ Space appears in Alice's panel
- ✅ Network graph shows Alice node
- ✅ CRDT timeline shows CreateSpace operation

Enjoy exploring your distributed system! 🚀
