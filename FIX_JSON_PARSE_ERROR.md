# Fix: JSON Parse Error on JoinSpace Action

## Problem

When Bob tried to join a Space, the frontend showed:

```
SyntaxError: Unexpected token 'F', "Failed to "... is not valid JSON
```

## Root Cause

**Backend/Frontend Mismatch:**

- **Frontend** was sending: `{ type: "JoinSpace", space_id: "..." }`
- **Backend** was expecting: `{ type: "JoinSpace", invite_code: "..." }`

When the backend tried to deserialize the JSON with `invite_code` field but received `space_id`, the deserialization failed and returned a plain text error instead of JSON, causing the "not valid JSON" error.

## Solution

Updated the backend to match the frontend's field name:

### 1. Action Enum (main.rs:56)

**Before:**

```rust
JoinSpace { invite_code: String },
```

**After:**

```rust
JoinSpace { space_id: String },
```

### 2. Action Handler (main.rs:323-339)

**Before:**

```rust
Action::JoinSpace { invite_code } => {
    // Parse invite_code as hex SpaceId
    let space_id_bytes = hex::decode(&invite_code)
        .map_err(|e| anyhow::anyhow!("Invalid invite_code hex: {}", e))?;

    if space_id_bytes.len() != 32 {
        return Err(anyhow::anyhow!("Invalid invite_code length, expected 32 bytes"));
    }
    // ...
}
```

**After:**

```rust
Action::JoinSpace { space_id } => {
    // Parse space_id as hex SpaceId
    let space_id_bytes = hex::decode(&space_id)
        .map_err(|e| anyhow::anyhow!("Invalid space_id hex: {}", e))?;

    if space_id_bytes.len() != 32 {
        return Err(anyhow::anyhow!("Invalid space_id length, expected 32 bytes (64 hex chars)"));
    }
    // ...
    Ok(format!("Joined space '{}' with ID: {}", space.name, hex::encode(&space.id.0[..8])))
}
```

### Improvements Made

- ✅ Fixed field name mismatch (`invite_code` → `space_id`)
- ✅ Improved error messages (mentions "64 hex chars")
- ✅ Better success message (includes space name)
- ✅ Consistent naming across frontend and backend

## Testing

Now when Bob tries to join:

1. **Frontend sends:**

```json
{
  "client": "bob",
  "action": {
    "type": "JoinSpace",
    "space_id": "eb2798d3bae58d5a..."
  }
}
```

2. **Backend successfully parses** the request

3. **Error response (if nodes not connected):**

```json
{
  "success": false,
  "message": "Action failed: Space not found or you don't have access",
  "data": null
}
```

4. **Success response (if it works):**

```json
{
  "success": true,
  "message": "Joined space 'red' with ID: eb2798d3",
  "data": null
}
```

## Next Steps

Bob will still get "Space not found" error because of the network connectivity issue (nodes aren't connected), but now:

- ✅ The error is properly displayed in JSON format
- ✅ The frontend can show the error message
- ✅ No more JSON parse errors

To actually make Bob join successfully, we need to implement one of the solutions from `NETWORK_CONNECTIVITY_ISSUE.md`:

- Option 1: Manual peer connection
- Option 2: In-memory transport
- Option 3: Bootstrap node
- Option 4: Direct space data sharing (recommended for dashboard)
