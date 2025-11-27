<!-- Use this file to provide workspace-specific custom instructions to Copilot. For more details, visit https://code.visualstudio.com/docs/copilot/copilot-customization#_use-a-githubcopilotinstructionsmd-file -->

# Discord-Lite Dashboard Development Guidelines

## Project Overview

This is a standalone dashboard for visualizing the Discord-Lite distributed forum system. It consists of:

- **Backend**: Rust + Axum + WebSocket server managing 3 Discord-Lite clients (Alice, Bob, Charlie)
- **Frontend**: React + Vite + TypeScript with real-time visualization

## Tech Stack

- **Backend**: Rust (nightly), Axum, tokio, serde, tower-http
- **Frontend**: React 19, TypeScript, Vite, custom CSS (no UI libraries)
- **Communication**: WebSocket for real-time updates, REST for actions

## Code Style

### Rust (Backend)

- Use `async/await` for all I/O operations
- Lock clients (`Arc<Mutex<Client>>`) only when needed, release quickly
- Add `tracing::info!` for important events
- Return proper `Result<T, String>` from action handlers
- Keep WebSocket update interval at 500ms (configurable if needed)

### TypeScript (Frontend)

- Use functional components with hooks
- Export all interfaces from `App.tsx` for component imports
- Use explicit types (avoid `any`)
- CSS modules or separate `.css` files per component
- WebSocket connection in useEffect with cleanup

## Architecture Patterns

### Backend State Management

```rust
AppState {
    alice: Arc<Mutex<Client>>,
    bob: Arc<Mutex<Client>>,
    charlie: Arc<Mutex<Client>>,
    state_snapshot: Arc<RwLock<DashboardState>>,
}
```

- Background task updates snapshot every 500ms
- WebSocket handler streams from snapshot
- Action handlers modify clients directly

### Frontend Data Flow

```
WebSocket → setState → Re-render components
User Action → POST /api/action → Backend updates → WebSocket update
```

## Key Interfaces

### DashboardState

```typescript
interface DashboardState {
  clients: ClientSnapshot[];
  network_graph: NetworkGraph;
  crdt_timeline: CrdtOperation[];
}
```

### Action Format

```typescript
interface ActionRequest {
  client: "alice" | "bob" | "charlie";
  action: Action; // CreateSpace, CreateChannel, etc.
}
```

## Component Structure

- `ClientPanel`: Shows space, channels, members for one client
- `ActionPanel`: Form to execute operations
- `NetworkGraph`: SVG visualization of peer connections
- `CrdtTimeline`: Scrollable operation feed

## Best Practices

1. **Performance**: Don't block the WebSocket loop - use separate tasks
2. **Error Handling**: Show errors in UI, don't crash
3. **UX**: Provide visual feedback for all actions
4. **Debugging**: Console.log WebSocket messages for troubleshooting
5. **Styling**: Dark theme (GitHub-inspired colors)

## Common Tasks

### Adding a New Action

1. Add variant to `Action` enum (backend)
2. Implement in `execute_action()` (backend)
3. Add form fields to `ActionPanel` (frontend)
4. Update `executeAction()` switch statement (frontend)

### Adding State Field

1. Add to `DashboardState` struct (backend)
2. Update `build_client_snapshot()` to populate it (backend)
3. Add TypeScript interface (frontend `App.tsx`)
4. Update component to display it

### Debugging Tips

- Backend logs: Look for `tracing::info!` output
- Frontend: Check browser console for WebSocket messages
- Network tab: Inspect WebSocket frames and POST requests
- Use `console.log(state)` to inspect current data

## Dependencies

- Backend depends on `descord-core` (path = "../../core")
- Frontend has no external UI libraries (pure React + CSS)
- Communication is JSON over WebSocket/HTTP

## Testing Workflow

1. Start backend: `cd dashboard-backend && cargo run`
2. Start frontend: `cd dashboard-frontend && npm run dev`
3. Open http://localhost:5173
4. Create space with Alice → Invite Bob → Observe state changes
