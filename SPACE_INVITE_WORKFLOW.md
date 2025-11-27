# 🌟 Complete Space & Invite Workflow Guide

## 📊 Overview: Space ID Format

**CRITICAL:** A Space ID is **64 hexadecimal characters** (32 bytes), NOT 8!

```
Example full Space ID:
eb2798d3bae58d5a1f2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a

What you saw (shortened):   eb2798d3
What you need (full):       eb2798d3bae58d5a1f2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a
                            ^^^^^^^^ ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                            8 chars  + 56 more characters = 64 total
```

The backend logs show `eb2798d3bae58d5a` (16 chars) because it's displaying only the first 8 bytes for brevity.

---

## 🚀 Complete Workflow: Alice Creates Space → Bob Joins

### Step 1️⃣: Alice Creates a Space

**In the Dashboard:**

- **Client:** Alice
- **Action:** Create Space
- **Space Name:** "red" (or any name you want)
- Click **Execute**

**Expected Response:**

```
✓ Created space 'red' with ID: eb2798d3
```

**⚠️ IMPORTANT:** The displayed ID `eb2798d3` is just the **first 8 bytes**!

**Finding the Full Space ID:**

Look at your dashboard - in the **Client Panels** section:

- Alice's panel should show the new space "red"
- The space will have a full ID displayed (or you can check the WebSocket data)

**Alternative - Check Backend Logs:**

```bash
# Look for this line in the terminal:
📢 [BROADCAST START] Broadcasting operation on topic: space/eb2798d3bae58d5a
                                                            ^^^^^^^^^^^^^^^^
# This shows the first 16 hex characters (8 bytes)
# But you need all 64 characters (32 bytes)
```

**Getting the Full ID Programmatically:**

The full Space ID is actually generated from:

```rust
SpaceId::from_content(&user_id, &name, timestamp)
```

For the dashboard, you need to extract it from the dashboard state.

---

### Step 2️⃣: Alice Creates an Invite

**In the Dashboard:**

- **Client:** Alice (same client that created the space)
- **Action:** Create Invite
- **Space ID:** Paste the **FULL 64-character hex string**
  - ❌ WRONG: `eb2798d3` (only 8 chars)
  - ❌ WRONG: `eb2798d3bae58d5a` (only 16 chars)
  - ✅ RIGHT: `eb2798d3bae58d5a1f2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a` (64 chars)

**Expected Response:**

```
✓ Created invite! Code: ABC123XY (Space: eb2798d3)
```

The invite code `ABC123XY` is an 8-character alphanumeric code that can be shared.

---

### Step 3️⃣: Bob Joins the Space

**In the Dashboard:**

- **Client:** Bob
- **Action:** Join Space
- **Invite Code:** The **FULL Space ID** (64 hex characters)

**Expected Response:**

```
✓ Joined space with ID: eb2798d3
```

After this, Bob should appear in the space's member list!

---

## 🔍 How to Get the Full Space ID

Since the current dashboard only shows the first 8 bytes, here are your options:

### Option 1: Check the WebSocket Stream (Recommended)

1. Open browser DevTools (F12)
2. Go to the **Network** tab
3. Filter for "ws" (WebSocket)
4. Click on the WebSocket connection
5. Go to **Messages** tab
6. Look for the latest message after creating the space
7. Find the space object with the full `id` field

Example WebSocket message:

```json
{
  "clients": [
    {
      "name": "Alice",
      "spaces": [
        {
          "id": "eb2798d3bae58d5a1f2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a",
          "name": "red",
          "owner": "290e150a5d221326...",
          "members": [...]
        }
      ]
    }
  ]
}
```

### Option 2: Add a REST Endpoint (Quick Fix)

I can add a `/api/spaces` endpoint to the backend that lists all spaces with full IDs.

### Option 3: Update the Frontend

I can modify the frontend to display and allow copying the full Space ID.

---

## 🧩 Understanding the Architecture

### Space ID Generation

```
SpaceId = Blake3(user_id || space_name || timestamp)
         ↓
         32 bytes (256 bits)
         ↓
         64 hex characters
```

### Invite Code vs Space ID

| Type            | Length   | Format                  | Purpose                         |
| --------------- | -------- | ----------------------- | ------------------------------- |
| **Space ID**    | 64 chars | Hex (0-9, a-f)          | Unique identifier for the space |
| **Invite Code** | 8 chars  | Alphanumeric (A-Z, 0-9) | Human-friendly code for sharing |

**Important:** Currently, the "Join Space" action expects the **Space ID**, not the short invite code.

### The Invite Flow

```
┌─────────────────────────────────────────────────────────────┐
│ 1. Alice creates Space "red"                                │
│    → Space ID: eb2798d3bae58d5a... (64 chars)              │
│    → Alice becomes owner/admin                              │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 2. Alice creates Invite for Space                           │
│    → Invite Code: ABC123XY (8 chars)                       │
│    → Invite links to Space ID                               │
│    → Backend stores: {code: "ABC123XY", space_id: "eb..."} │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 3. Bob receives Space ID from Alice                         │
│    (In production: Alice shares the 8-char invite code)     │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 4. Bob joins using Space ID                                 │
│    → Backend validates Bob has access                       │
│    → Checks invite exists and is valid                      │
│    → Adds Bob to space members                              │
│    → Bob receives MLS Welcome message (E2EE setup)          │
└─────────────────────────────────────────────────────────────┘
                          ↓
┌─────────────────────────────────────────────────────────────┐
│ 5. Both Alice and Bob can now:                              │
│    → See the space in their dashboard                       │
│    → Create channels in the space                           │
│    → Send encrypted messages                                │
└─────────────────────────────────────────────────────────────┘
```

---

## 🛠️ Quick Fix: Get Full Space ID from Dashboard

Let me add a helper endpoint to make this easier. Run this in the dashboard:

```bash
curl http://localhost:3030/api/state | jq '.clients[0].spaces[0].id'
```

This will show Alice's first space's full ID.

---

## 🔧 Current Dashboard Limitations

### What Works:

✅ Alice creates space  
✅ Alice creates invite (if you have the full 64-char Space ID)  
✅ Bob joins space (if you have the full 64-char Space ID)  
✅ Real-time updates via WebSocket

### What Needs Full Space ID:

⚠️ Create Invite action  
⚠️ Join Space action  
⚠️ Create Channel action

### Workaround:

**Option A:** Use browser DevTools to copy the full Space ID from WebSocket messages

**Option B:** I can add a "Copy Space ID" button to the frontend

**Option C:** Use curl to fetch the full ID:

```bash
# Get Alice's spaces
curl -s http://localhost:3030/api/state | jq '.clients[0].spaces'

# Get the first space's ID
curl -s http://localhost:3030/api/state | jq -r '.clients[0].spaces[0].id'
```

---

## 📝 Step-by-Step Example (With Terminal Commands)

### 1. Alice creates space "red"

**Dashboard:** Alice → Create Space → Name: "red" → Execute

### 2. Get the full Space ID

```bash
# In a terminal:
SPACE_ID=$(curl -s http://localhost:3030/api/state | jq -r '.clients[0].spaces[0].id')
echo "Full Space ID: $SPACE_ID"
```

Expected output:

```
Full Space ID: eb2798d3bae58d5a1f2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a
```

### 3. Alice creates invite (using API directly)

```bash
curl -X POST http://localhost:3030/api/action \
  -H "Content-Type: application/json" \
  -d "{
    \"client\": \"alice\",
    \"action\": {
      \"type\": \"CreateInvite\",
      \"space_id\": \"$SPACE_ID\"
    }
  }"
```

Expected response:

```json
{
  "success": true,
  "message": "Created invite! Code: ABC123XY (Space: eb2798d3)",
  "data": null
}
```

### 4. Bob joins (using API directly)

```bash
curl -X POST http://localhost:3030/api/action \
  -H "Content-Type: application/json" \
  -d "{
    \"client\": \"bob\",
    \"action\": {
      \"type\": \"JoinSpace\",
      \"invite_code\": \"$SPACE_ID\"
    }
  }"
```

Expected response:

```json
{
  "success": true,
  "message": "Joined space with ID: eb2798d3",
  "data": null
}
```

### 5. Verify Bob joined

```bash
curl -s http://localhost:3030/api/state | jq '.clients[1].spaces'
```

You should see the "red" space in Bob's space list!

---

## 🎯 What You Should See in the Dashboard

### After Alice creates the space:

**Alice's Panel:**

```
👩 Alice (290e150a)
  Spaces (1):
    • red (eb2798d3...)
      Members: 1
      Channels: 0
```

### After Alice creates invite:

**Success Message:**

```
✓ Created invite! Code: ABC123XY (Space: eb2798d3)
```

### After Bob joins:

**Bob's Panel:**

```
👨 Bob (958b3eb2)
  Spaces (1):
    • red (eb2798d3...)
      Members: 2  ← Now shows 2 members!
      Channels: 0
```

**Alice's Panel (updated):**

```
👩 Alice (290e150a)
  Spaces (1):
    • red (eb2798d3...)
      Members: 2  ← Updated to show Bob joined!
      Channels: 0
```

---

## 🐛 Troubleshooting

### Error: "Invalid space_id length, expected 32 bytes"

**Cause:** You entered a partial Space ID (like `eb2798d3` which is only 8 chars)

**Fix:** Enter the **full 64-character** hex Space ID

### Error: "Invalid space_id hex"

**Cause:** The Space ID contains non-hex characters

**Fix:** Only use characters 0-9 and a-f (lowercase)

### Error: "Space not found"

**Cause:** Bob is trying to create an invite for a space he doesn't have access to

**Fix:** Only the space owner/members can create invites. Use Alice's client.

### Error: "Insufficient permissions to create invites"

**Cause:** The user doesn't have permission (not an admin/moderator)

**Fix:** Make sure you're using the space owner's client (Alice in this case)

---

## 🚀 Next Steps

Would you like me to:

1. **Add a UI button** to copy the full Space ID to clipboard?
2. **Add a `/api/spaces` endpoint** to list all spaces with full IDs?
3. **Update the frontend** to display full Space IDs in the space panels?
4. **Implement short invite code** lookup (so Bob can join with just "ABC123XY")?

Let me know which would be most helpful!
