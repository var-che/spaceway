import { useState, useEffect } from "react";
import "./DiscordUI.css";

interface Space {
  id: string;
  name: string;
  channels?: Channel[];
  member_count?: number;
}

interface Channel {
  id: string;
  name: string;
  threads?: Thread[];
}

interface Thread {
  id: string;
  title: string;
  messages?: Message[];
}

interface Message {
  id: string;
  content: string;
  author: string;
  timestamp: number;
}

interface ApiMessage {
  id: string;
  content: string;
  author: string;
  created_at: number;
}

interface ApiThread {
  id: string;
  title: string | null;
  creator: string;
  created_at: number;
  message_count: number;
  messages: ApiMessage[];
}

interface ApiChannel {
  id: string;
  name: string;
  threads: ApiThread[];
}

interface DashboardApiClient {
  name: string;
  user_id: string;
  spaces: {
    id: string;
    name: string;
    channels?: ApiChannel[];
    member_count?: number;
  }[];
}

interface DashboardApiResponse {
  clients: DashboardApiClient[];
}

interface ClientSnapshot {
  name: string;
  user_id: string;
  spaces: Space[];
}

export default function DiscordUI() {
  const [activeClient, setActiveClient] = useState<"alice" | "bob" | "charlie">(
    "alice"
  );
  const [clientData, setClientData] = useState<Record<string, ClientSnapshot>>(
    {}
  );
  const [selectedSpace, setSelectedSpace] = useState<Space | null>(null);
  const [selectedChannel, setSelectedChannel] = useState<Channel | null>(null);
  const [selectedThread, setSelectedThread] = useState<Thread | null>(null);

  // Modal states
  const [showCreateSpace, setShowCreateSpace] = useState(false);
  const [showCreateChannel, setShowCreateChannel] = useState(false);
  const [showCreateThread, setShowCreateThread] = useState(false);
  const [showJoinSpace, setShowJoinSpace] = useState(false);
  const [showKickMember, setShowKickMember] = useState(false);

  // Form states
  const [spaceName, setSpaceName] = useState("");
  const [channelName, setChannelName] = useState("");
  const [threadTitle, setThreadTitle] = useState("");
  const [firstMessage, setFirstMessage] = useState("");
  const [messageContent, setMessageContent] = useState("");
  const [spaceIdToJoin, setSpaceIdToJoin] = useState("");
  const [userIdToKick, setUserIdToKick] = useState("");

  // Notification state
  const [notification, setNotification] = useState<{
    type: "success" | "error";
    message: string;
  } | null>(null);

  // Fetch dashboard state
  useEffect(() => {
    const fetchState = async () => {
      try {
        const response = await fetch("http://localhost:3030/api/state");
        const data: DashboardApiResponse = await response.json();

        const clientsData: Record<string, ClientSnapshot> = {};
        data.clients.forEach((client: DashboardApiClient) => {
          clientsData[client.name.toLowerCase()] = {
            name: client.name,
            user_id: client.user_id,
            spaces: client.spaces.map((space) => ({
              id: space.id,
              name: space.name,
              channels: (space.channels || []).map((channel: ApiChannel) => ({
                id: channel.id,
                name: channel.name,
                threads: (channel.threads || []).map((thread: ApiThread) => ({
                  id: thread.id,
                  title: thread.title || "Untitled",
                  messages: (thread.messages || []).map((msg: ApiMessage) => ({
                    id: msg.id,
                    content: msg.content,
                    author: msg.author,
                    timestamp: msg.created_at || Date.now(),
                  })),
                })),
              })),
              member_count: space.member_count || 1,
            })),
          };
        });

        setClientData(clientsData);
      } catch (error) {
        console.error("Failed to fetch state:", error);
      }
    };

    fetchState();
    const interval = setInterval(fetchState, 1000);
    return () => clearInterval(interval);
  }, []);

  const showNotification = (type: "success" | "error", message: string) => {
    setNotification({ type, message });
    setTimeout(() => setNotification(null), 5000);
  };

  const executeAction = async (action: Record<string, unknown>) => {
    try {
      const response = await fetch("http://localhost:3030/api/action", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ client: activeClient, action }),
      });

      const data = await response.json();

      if (data.success) {
        showNotification("success", data.message);
        return data;
      } else {
        showNotification("error", data.message || "Action failed");
        return null;
      }
    } catch (error) {
      showNotification("error", `Network error: ${error}`);
      return null;
    }
  };

  const handleCreateSpace = async () => {
    if (!spaceName.trim()) {
      showNotification("error", "Space name cannot be empty");
      return;
    }

    await executeAction({ type: "CreateSpace", name: spaceName });
    setSpaceName("");
    setShowCreateSpace(false);
  };

  const handleCreateChannel = async () => {
    if (!channelName.trim()) {
      showNotification("error", "Channel name cannot be empty");
      return;
    }
    if (!selectedSpace) {
      showNotification("error", "No space selected");
      return;
    }

    const result = await executeAction({
      type: "CreateChannel",
      space_id: selectedSpace.id,
      name: channelName,
    });

    if (result) {
      setChannelName("");
      setShowCreateChannel(false);
      // Give WebSocket time to update before user interacts again
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  };

  const handleCreateThread = async () => {
    if (!firstMessage.trim()) {
      showNotification("error", "First message cannot be empty");
      return;
    }
    if (!selectedChannel) {
      showNotification("error", "No channel selected");
      return;
    }

    await executeAction({
      type: "CreateThread",
      space_id: selectedSpace!.id,
      channel_id: selectedChannel.id,
      title: threadTitle || null,
      first_message: firstMessage,
    });
    setThreadTitle("");
    setFirstMessage("");
    setShowCreateThread(false);
  };

  const handleSendMessage = async () => {
    if (!messageContent.trim()) {
      showNotification("error", "Message cannot be empty");
      return;
    }
    if (!selectedThread) {
      showNotification("error", "No thread selected");
      return;
    }

    await executeAction({
      type: "SendMessage",
      space_id: selectedSpace!.id,
      thread_id: selectedThread.id,
      content: messageContent,
    });
    setMessageContent("");
  };

  const handleJoinSpace = async () => {
    if (!spaceIdToJoin.trim()) {
      showNotification("error", "Space ID cannot be empty");
      return;
    }

    await executeAction({
      type: "JoinSpace",
      space_id: spaceIdToJoin,
    });
    setSpaceIdToJoin("");
    setShowJoinSpace(false);
  };

  const handleKickMember = async () => {
    if (!userIdToKick.trim()) {
      showNotification("error", "User ID cannot be empty");
      return;
    }
    if (!selectedSpace) {
      showNotification("error", "No space selected");
      return;
    }

    await executeAction({
      type: "RemoveMember",
      space_id: selectedSpace.id,
      user_id: userIdToKick,
    });
    setUserIdToKick("");
    setShowKickMember(false);
  };

  const handleConnectPeers = async () => {
    await executeAction({ type: "ConnectPeers" });
  };

  const currentClient = clientData[activeClient];

  return (
    <div className="discord-ui">
      {/* Client Selector Bar */}
      <div className="client-selector">
        <button
          className={`client-btn ${activeClient === "alice" ? "active" : ""}`}
          onClick={() => setActiveClient("alice")}
        >
          👩 Alice
        </button>
        <button
          className={`client-btn ${activeClient === "bob" ? "active" : ""}`}
          onClick={() => setActiveClient("bob")}
        >
          👨 Bob
        </button>
        <button
          className={`client-btn ${activeClient === "charlie" ? "active" : ""}`}
          onClick={() => setActiveClient("charlie")}
        >
          🧑 Charlie
        </button>
        <button className="connect-peers-btn" onClick={handleConnectPeers}>
          🔗 Connect Peers
        </button>
      </div>

      <div className="discord-container">
        {/* Left Sidebar - Spaces */}
        <div className="spaces-sidebar">
          <div className="sidebar-header">
            <h3>Spaces</h3>
            <button
              className="icon-btn"
              onClick={() => setShowCreateSpace(true)}
              title="Create Space"
            >
              ➕
            </button>
          </div>

          <div className="spaces-list">
            {currentClient?.spaces.map((space) => (
              <div
                key={space.id}
                className={`space-item ${
                  selectedSpace?.id === space.id ? "selected" : ""
                }`}
                onClick={() => {
                  setSelectedSpace(space);
                  setSelectedChannel(null);
                  setSelectedThread(null);
                }}
              >
                <div className="space-name">{space.name}</div>
                <div className="space-meta">
                  👥 {space.member_count || 1} | 📝{" "}
                  {space.channels?.length || 0}
                </div>
              </div>
            ))}
          </div>

          <button
            className="join-space-btn"
            onClick={() => setShowJoinSpace(true)}
          >
            Join Space
          </button>
        </div>

        {/* Middle Sidebar - Channels */}
        <div className="channels-sidebar">
          {selectedSpace ? (
            <>
              <div className="sidebar-header">
                <h3>{selectedSpace.name}</h3>
                <div className="space-actions">
                  <button
                    className="icon-btn"
                    onClick={() => setShowCreateChannel(true)}
                    title="Create Channel"
                  >
                    ➕
                  </button>
                  <button
                    className="icon-btn danger"
                    onClick={() => setShowKickMember(true)}
                    title="Kick Member"
                  >
                    🚫
                  </button>
                </div>
              </div>

              <div className="space-info">
                <div className="info-item">
                  <span className="label">Space ID:</span>
                  <span className="value small">
                    {selectedSpace.id.slice(0, 16)}...
                  </span>
                  <button
                    className="copy-btn"
                    onClick={() => {
                      navigator.clipboard.writeText(selectedSpace.id);
                      showNotification("success", "Space ID copied!");
                    }}
                  >
                    📋
                  </button>
                </div>
                <div className="info-item">
                  <span className="label">Members:</span>
                  <span className="value">
                    {selectedSpace.member_count || 1}
                  </span>
                </div>
              </div>

              <div className="channels-list">
                <div className="section-title">CHANNELS</div>
                {selectedSpace.channels?.map((channel) => (
                  <div
                    key={channel.id}
                    className={`channel-item ${
                      selectedChannel?.id === channel.id ? "selected" : ""
                    }`}
                    onClick={() => {
                      setSelectedChannel(channel);
                      setSelectedThread(null);
                    }}
                  >
                    # {channel.name}
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="empty-state">
              <p>Select a space to view channels</p>
            </div>
          )}
        </div>

        {/* Right Sidebar - Threads */}
        <div className="threads-sidebar">
          {selectedChannel ? (
            <>
              <div className="sidebar-header">
                <h3># {selectedChannel.name}</h3>
                <button
                  className="icon-btn"
                  onClick={() => setShowCreateThread(true)}
                  title="Create Thread"
                >
                  ➕
                </button>
              </div>

              <div className="threads-list">
                <div className="section-title">THREADS</div>
                {selectedChannel.threads?.map((thread) => (
                  <div
                    key={thread.id}
                    className={`thread-item ${
                      selectedThread?.id === thread.id ? "selected" : ""
                    }`}
                    onClick={() => setSelectedThread(thread)}
                  >
                    💬 {thread.title || "Untitled"}
                  </div>
                ))}
              </div>
            </>
          ) : (
            <div className="empty-state">
              <p>Select a channel to view threads</p>
            </div>
          )}
        </div>

        {/* Main Content - Messages */}
        <div className="main-content">
          {selectedThread ? (
            <>
              <div className="content-header">
                <h2>💬 {selectedThread.title || "Untitled Thread"}</h2>
                <div className="thread-info">
                  Thread ID: {selectedThread.id.slice(0, 16)}...
                </div>
              </div>

              <div className="messages-container">
                {selectedThread.messages?.map((msg) => (
                  <div key={msg.id} className="message">
                    <div className="message-author">{msg.author}</div>
                    <div className="message-content">{msg.content}</div>
                  </div>
                ))}
                {(!selectedThread.messages ||
                  selectedThread.messages.length === 0) && (
                  <div className="empty-messages">
                    No messages yet. Start the conversation!
                  </div>
                )}
              </div>

              <div className="message-input">
                <input
                  type="text"
                  placeholder="Type a message..."
                  value={messageContent}
                  onChange={(e) => setMessageContent(e.target.value)}
                  onKeyPress={(e) => e.key === "Enter" && handleSendMessage()}
                />
                <button onClick={handleSendMessage}>Send</button>
              </div>
            </>
          ) : (
            <div className="empty-state">
              <h2>Welcome to Spaceway</h2>
              <p>Select a thread to start messaging</p>
              <div className="quick-actions">
                <button onClick={() => setShowCreateSpace(true)}>
                  Create a Space
                </button>
                <button onClick={() => setShowJoinSpace(true)}>
                  Join a Space
                </button>
                <button onClick={handleConnectPeers}>Connect Peers</button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Notifications */}
      {notification && (
        <div className={`notification ${notification.type}`}>
          {notification.type === "success" ? "✓" : "✗"} {notification.message}
        </div>
      )}

      {/* Modals */}
      {showCreateSpace && (
        <Modal title="Create Space" onClose={() => setShowCreateSpace(false)}>
          <input
            type="text"
            placeholder="Space name"
            value={spaceName}
            onChange={(e) => setSpaceName(e.target.value)}
            onKeyPress={(e) => e.key === "Enter" && handleCreateSpace()}
            autoFocus
          />
          <div className="modal-actions">
            <button onClick={handleCreateSpace}>Create</button>
            <button onClick={() => setShowCreateSpace(false)}>Cancel</button>
          </div>
        </Modal>
      )}

      {showCreateChannel && (
        <Modal
          title="Create Channel"
          onClose={() => setShowCreateChannel(false)}
        >
          <input
            type="text"
            placeholder="Channel name"
            value={channelName}
            onChange={(e) => setChannelName(e.target.value)}
            onKeyPress={(e) => e.key === "Enter" && handleCreateChannel()}
            autoFocus
          />
          <div className="modal-actions">
            <button onClick={handleCreateChannel}>Create</button>
            <button onClick={() => setShowCreateChannel(false)}>Cancel</button>
          </div>
        </Modal>
      )}

      {showCreateThread && (
        <Modal title="Create Thread" onClose={() => setShowCreateThread(false)}>
          <input
            type="text"
            placeholder="Thread title (optional)"
            value={threadTitle}
            onChange={(e) => setThreadTitle(e.target.value)}
          />
          <textarea
            placeholder="First message"
            value={firstMessage}
            onChange={(e) => setFirstMessage(e.target.value)}
            autoFocus
          />
          <div className="modal-actions">
            <button onClick={handleCreateThread}>Create</button>
            <button onClick={() => setShowCreateThread(false)}>Cancel</button>
          </div>
        </Modal>
      )}

      {showJoinSpace && (
        <Modal title="Join Space" onClose={() => setShowJoinSpace(false)}>
          <input
            type="text"
            placeholder="Space ID (64 characters)"
            value={spaceIdToJoin}
            onChange={(e) => setSpaceIdToJoin(e.target.value)}
            autoFocus
          />
          <div className="modal-actions">
            <button onClick={handleJoinSpace}>Join</button>
            <button onClick={() => setShowJoinSpace(false)}>Cancel</button>
          </div>
        </Modal>
      )}

      {showKickMember && (
        <Modal title="Kick Member" onClose={() => setShowKickMember(false)}>
          <input
            type="text"
            placeholder="User ID (64 characters)"
            value={userIdToKick}
            onChange={(e) => setUserIdToKick(e.target.value)}
            autoFocus
          />
          <div className="help-text">
            Get user IDs from backend logs or other client panels
          </div>
          <div className="modal-actions">
            <button className="danger" onClick={handleKickMember}>
              Kick
            </button>
            <button onClick={() => setShowKickMember(false)}>Cancel</button>
          </div>
        </Modal>
      )}
    </div>
  );
}

function Modal({
  title,
  children,
  onClose,
}: {
  title: string;
  children: React.ReactNode;
  onClose: () => void;
}) {
  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h3>{title}</h3>
          <button className="close-btn" onClick={onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">{children}</div>
      </div>
    </div>
  );
}
