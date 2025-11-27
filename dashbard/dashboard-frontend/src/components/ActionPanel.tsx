import { useState } from "react";
import "./ActionPanel.css";

export default function ActionPanel() {
  const [client, setClient] = useState("alice");
  const [actionType, setActionType] = useState("CreateSpace");
  const [spaceName, setSpaceName] = useState("");
  const [spaceId, setSpaceId] = useState("");
  const [channelId, setChannelId] = useState("");
  const [channelName, setChannelName] = useState("");
  const [threadId, setThreadId] = useState("");
  const [threadTitle, setThreadTitle] = useState("");
  const [message, setMessage] = useState("");
  const [userId, setUserId] = useState("");
  const [result, setResult] = useState<{
    success: boolean;
    message?: string;
    error?: string;
  } | null>(null);

  const executeAction = async () => {
    const action: Record<string, unknown> = { type: actionType };

    switch (actionType) {
      case "CreateSpace":
        action.name = spaceName;
        break;
      case "CreateChannel":
        action.space_id = spaceId;
        action.name = channelName;
        break;
      case "CreateThread":
        action.space_id = spaceId;
        action.channel_id = channelId;
        action.title = threadTitle || null;
        action.first_message = message;
        break;
      case "SendMessage":
        action.space_id = spaceId;
        action.thread_id = threadId;
        action.content = message;
        break;
      case "CreateInvite":
        action.space_id = spaceId;
        break;
      case "JoinSpace":
        action.space_id = spaceId;
        break;
      case "RemoveMember":
        action.space_id = spaceId;
        action.user_id = userId;
        break;
    }

    try {
      const response = await fetch("http://localhost:3030/api/action", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ client, action }),
      });

      const data = await response.json();
      setResult(data);

      // Clear inputs on success
      if (data.success) {
        setSpaceName("");
        setChannelName("");
        setSpaceId("");
        setChannelId("");
        setThreadId("");
        setThreadTitle("");
        setMessage("");
        setUserId("");
      }
    } catch (error) {
      setResult({ success: false, error: String(error) });
    }
  };

  return (
    <div className="action-panel-content">
      <div className="form-group">
        <label>Client:</label>
        <select value={client} onChange={(e) => setClient(e.target.value)}>
          <option value="alice">👩 Alice</option>
          <option value="bob">👨 Bob</option>
          <option value="charlie">🧑 Charlie</option>
        </select>
      </div>

      <div className="form-group">
        <label>Action:</label>
        <select
          value={actionType}
          onChange={(e) => setActionType(e.target.value)}
        >
          <option value="CreateSpace">Create Space</option>
          <option value="CreateChannel">Create Channel</option>
          <option value="CreateThread">Create Thread</option>
          <option value="SendMessage">Send Message</option>
          <option value="CreateInvite">Create Invite</option>
          <option value="JoinSpace">Join Space</option>
          <option value="RemoveMember">🚫 Remove Member (Kick)</option>
          <option value="ConnectPeers">🔗 Connect Peers (P2P Network)</option>
        </select>
      </div>

      {actionType === "CreateSpace" && (
        <div className="form-group">
          <label>Space Name:</label>
          <input
            type="text"
            value={spaceName}
            onChange={(e) => setSpaceName(e.target.value)}
            placeholder="My Awesome Space"
          />
        </div>
      )}

      {actionType === "CreateChannel" && (
        <>
          <div className="form-group">
            <label>Space ID:</label>
            <input
              type="text"
              value={spaceId}
              onChange={(e) => setSpaceId(e.target.value)}
              placeholder="UUID of space"
            />
          </div>
          <div className="form-group">
            <label>Channel Name:</label>
            <input
              type="text"
              value={channelName}
              onChange={(e) => setChannelName(e.target.value)}
              placeholder="general"
            />
          </div>
        </>
      )}

      {actionType === "CreateInvite" && (
        <div className="form-group">
          <label>Space ID:</label>
          <input
            type="text"
            value={spaceId}
            onChange={(e) => setSpaceId(e.target.value)}
            placeholder="UUID of space"
          />
        </div>
      )}

      {actionType === "JoinSpace" && (
        <div className="form-group">
          <label>Space ID (64 chars):</label>
          <input
            type="text"
            value={spaceId}
            onChange={(e) => setSpaceId(e.target.value)}
            placeholder="Full 64-character Space ID"
          />
        </div>
      )}

      {actionType === "CreateThread" && (
        <>
          <div className="form-group">
            <label>Space ID:</label>
            <input
              type="text"
              value={spaceId}
              onChange={(e) => setSpaceId(e.target.value)}
              placeholder="Space ID (64 chars)"
            />
          </div>
          <div className="form-group">
            <label>Channel ID:</label>
            <input
              type="text"
              value={channelId}
              onChange={(e) => setChannelId(e.target.value)}
              placeholder="Channel ID (64 chars)"
            />
          </div>
          <div className="form-group">
            <label>Thread Title (optional):</label>
            <input
              type="text"
              value={threadTitle}
              onChange={(e) => setThreadTitle(e.target.value)}
              placeholder="Discussion Topic"
            />
          </div>
          <div className="form-group">
            <label>First Message:</label>
            <input
              type="text"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="Hello everyone!"
            />
          </div>
        </>
      )}

      {actionType === "SendMessage" && (
        <>
          <div className="form-group">
            <label>Space ID:</label>
            <input
              type="text"
              value={spaceId}
              onChange={(e) => setSpaceId(e.target.value)}
              placeholder="Space ID (64 chars)"
            />
          </div>
          <div className="form-group">
            <label>Thread ID:</label>
            <input
              type="text"
              value={threadId}
              onChange={(e) => setThreadId(e.target.value)}
              placeholder="Thread ID (64 chars)"
            />
          </div>
          <div className="form-group">
            <label>Message:</label>
            <input
              type="text"
              value={message}
              onChange={(e) => setMessage(e.target.value)}
              placeholder="Type your message..."
            />
          </div>
        </>
      )}

      {actionType === "RemoveMember" && (
        <>
          <div className="form-group">
            <label>Space ID:</label>
            <input
              type="text"
              value={spaceId}
              onChange={(e) => setSpaceId(e.target.value)}
              placeholder="Space ID (64 chars)"
            />
          </div>
          <div className="form-group">
            <label>User ID to Kick:</label>
            <input
              type="text"
              value={userId}
              onChange={(e) => setUserId(e.target.value)}
              placeholder="User ID (64 chars - full hex)"
            />
          </div>
        </>
      )}

      <button className="execute-btn" onClick={executeAction}>
        Execute
      </button>

      {result && (
        <div className={`result ${result.success ? "success" : "error"}`}>
          {result.success ? (
            <>✓ {result.message}</>
          ) : (
            <>
              <div className="error-header">✗ Error</div>
              <div className="error-message">
                {result.message || result.error || "Unknown error occurred"}
              </div>
            </>
          )}
        </div>
      )}
    </div>
  );
}
