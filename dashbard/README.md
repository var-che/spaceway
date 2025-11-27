# Discord-Lite Dashboard

A standalone visualization dashboard for the Discord-Lite distributed forum system. This dashboard allows you to observe and control 3 clients (Alice, Bob, Charlie) simultaneously, visualizing their internal state, network topology, CRDT operations, and distributed system behavior in real-time.

## 🎯 Current Status

**✅ Dashboard Infrastructure**: Fully functional

- Backend: Rust + Axum + WebSocket (running on http://127.0.0.1:3030)
- Frontend: React + Vite + TypeScript (running on http://localhost:5173)
- Communication: WebSocket streaming (500ms updates) + REST API

**⚠️ Integration Status**: Mock mode

- Dashboard works perfectly but uses mock data
- Actions return success messages but don't create real spaces/channels
- Waiting for `spaceway-core` API to be implemented

**📋 API Specification**: Complete

- Full specification documented in `SPACEWAY_CORE_API_SPEC.md`
- Implementation guide available in `IMPLEMENTATION_GUIDE.md`
- Ready for integration once spaceway-core APIs are available

## 📚 Documentation

| Document                                                     | Purpose                                      |
| ------------------------------------------------------------ | -------------------------------------------- |
| **[SPACEWAY_CORE_API_SPEC.md](./SPACEWAY_CORE_API_SPEC.md)** | Complete API specification for spaceway-core |
| **[IMPLEMENTATION_GUIDE.md](./IMPLEMENTATION_GUIDE.md)**     | Quick start guide with code examples         |
| **[LIVE.md](./LIVE.md)**                                     | Current deployment status and next steps     |
| **[QUICKSTART.md](./QUICKSTART.md)**                         | How to run and use the dashboard             |
| **[STATUS.md](./STATUS.md)**                                 | Setup history and troubleshooting            |

## 🚀 Quick Start

### Running the Dashboard (Mock Mode)

```bash
# Terminal 1: Backend
cd dashboard-backend
cargo run

# Terminal 2: Frontend (already running)
# Open http://localhost:5173
```

### Integrating Real Clients

1. **Read the specs**: Start with `IMPLEMENTATION_GUIDE.md`
2. **Add APIs to spaceway-core**: Implement methods in `core/src/client.rs`
3. **Update dashboard backend**: Replace mock clients with real ones
4. **Test**: Create spaces and watch them appear!

See `SPACEWAY_CORE_API_SPEC.md` for the complete API design.

## Architecture

```
dashboard/
├── dashboard-backend/     # Rust + Axum + WebSocket server
│   └── Manages 3 Discord-Lite clients
│       Exposes state via WebSocket API
│       Handles action requests from frontend
│
└── dashboard-frontend/    # React + Vite + TypeScript
    └── Real-time visualization
        Network graph, storage inspector
        Action controls, CRDT timeline
```

## Features

### Backend (`dashboard-backend`)

- **3-Client Orchestrator**: Manages Alice, Bob, and Charlie instances
- **WebSocket API**: Real-time state streaming (500ms updates)
- **REST API**: Execute actions (create space, join, send messages, etc.)
- **State Inspection**:
  - CRDT operation log
  - DHT storage entries
  - MLS group state (epochs, members)
  - Network topology (peers, connections)
  - Permission & role data

### Frontend (`dashboard-frontend`)

- **Network Graph**: Visualize peer connections (GossipSub, DHT)
- **Client Panels**: Side-by-side view of Alice, Bob, Charlie storage
- **Action Panel**: Interactive controls to trigger operations
- **CRDT Timeline**: Chronological operation feed
- **Real-time Updates**: WebSocket-driven live updates

## Getting Started

### Prerequisites

- **Node.js**: 20.19+ or 22.12+ (for frontend)
- **Rust**: nightly toolchain (for backend)
- **Discord-Lite core**: Must be built first

### 1. Build the Backend

```bash
cd dashboard-backend
cargo build --release
```

### 2. Install Frontend Dependencies

```bash
cd dashboard-frontend
npm install
```

### 3. Run the System

**Terminal 1 - Start Backend:**

```bash
cd dashboard-backend
cargo run
```

The backend will start on `http://localhost:3030`

**Terminal 2 - Start Frontend:**

```bash
cd dashboard-frontend
npm run dev
```

The frontend will start on `http://localhost:5173`

### 4. Open Dashboard

Navigate to `http://localhost:5173` in your browser.

## Usage

### Creating a Space

1. Select a client (Alice, Bob, or Charlie)
2. Choose "Create Space" action
3. Enter a space name
4. Click "Execute"
5. Watch the space appear in the client's panel

### Inviting Members

1. Select the space owner (e.g., Alice)
2. Choose "Create Invite" action
3. Copy the space ID from Alice's panel
4. Click "Execute" to generate an invite
5. Copy the invite code from the response
6. Select another client (e.g., Bob)
7. Choose "Join Space" action
8. Paste the invite code
9. Click "Execute"
10. Watch Bob join Alice's space

### Observing the System

**Network Graph**: Shows connections between clients. Solid lines = GossipSub, dashed = DHT.

**Client Panels**: Each panel shows:

- User ID
- Spaces joined
- Space members & their roles
- Channels in each space
- Permission assignments

**CRDT Timeline**: Shows every operation in chronological order with author, type, and affected resources.

## API Reference

### WebSocket Endpoint

- **URL**: `ws://localhost:3030/ws`
- **Protocol**: JSON messages every 500ms
- **Message Format**:

```json
{
  "clients": [...],
  "network_graph": { "nodes": [...], "edges": [...] },
  "crdt_timeline": [...]
}
```

### REST Endpoints

#### Execute Action

- **POST** `/api/action`
- **Body**:

```json
{
  "client": "alice" | "bob" | "charlie",
  "action": {
    "type": "CreateSpace",
    "name": "My Space"
  }
}
```

#### Get State Snapshot

- **GET** `/api/state`
- **Response**: Current dashboard state (same as WebSocket)

## Action Types

```typescript
CreateSpace: { name: string }
CreateChannel: { space_id: string, name: string }
CreateInvite: { space_id: string }
JoinSpace: { invite_code: string }
SendMessage: { channel_id: string, content: string }
```

## Development

### Frontend Development

```bash
cd dashboard-frontend
npm run dev      # Start dev server
npm run build    # Build for production
npm run lint     # Run ESLint
```

### Backend Development

```bash
cd dashboard-backend
cargo run        # Run in debug mode
cargo build --release  # Build optimized binary
cargo check      # Quick type-checking
```

## Troubleshooting

**Frontend shows "Connecting to backend..."**

- Ensure backend is running on port 3030
- Check browser console for WebSocket errors

**Backend fails to start**

- Ensure Discord-Lite core is built: `cd ../../core && cargo build`
- Check that port 3030 is available

**Network graph is empty**

- Create a space first - nodes/edges appear when clients share spaces

**Actions fail**

- Check backend console logs for detailed error messages
- Ensure UUIDs are valid when copying space/channel IDs

## Architecture Details

### State Update Flow

```
Backend (500ms interval)
  ├─> Lock Alice, Bob, Charlie clients
  ├─> Extract spaces, channels, members
  ├─> Build network graph
  ├─> Serialize to JSON
  └─> Broadcast via WebSocket

Frontend
  ├─> Receive WebSocket message
  ├─> Update React state
  └─> Re-render all panels
```

### Action Execution Flow

```
Frontend
  └─> POST /api/action

Backend
  ├─> Parse request
  ├─> Lock target client
  ├─> Execute Client method
  ├─> Return result
  └─> Next state update includes changes
```

## Future Enhancements

- [ ] Pause/Resume state updates
- [ ] Filter CRDT timeline by operation type
- [ ] Export state snapshots
- [ ] Message threading visualization
- [ ] DHT storage inspector (key/value viewer)
- [ ] MLS group details (proposals, commits)
- [ ] Network latency simulation
- [ ] Operation playback (step through CRDT ops)

## License

Same as Discord-Lite core project.
