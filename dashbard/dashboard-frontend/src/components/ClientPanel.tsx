import type { ClientSnapshot } from "../App";
import "./ClientPanel.css";

interface ClientPanelProps {
  client: ClientSnapshot;
}

export default function ClientPanel({ client }: ClientPanelProps) {
  if (!client) {
    return <div className="client-panel-content">No data</div>;
  }

  return (
    <div className="client-panel-content">
      <div className="client-info">
        <div className="info-row">
          <span className="label">User ID:</span>
          <span className="value">{client.user_id}</span>
        </div>
        <div className="info-row">
          <span className="label">Spaces:</span>
          <span className="value">{client.spaces.length}</span>
        </div>
      </div>

      <div className="spaces-section">
        <h3>Spaces</h3>
        {client.spaces.length === 0 ? (
          <p className="empty">No spaces yet</p>
        ) : (
          client.spaces.map((space) => (
            <div key={space.id} className="space-card">
              <div className="space-header">
                <h4>{space.name}</h4>
                <button
                  className="copy-id-btn"
                  onClick={() => {
                    navigator.clipboard.writeText(space.id);
                    alert(`Copied Space ID:\n${space.id}`);
                  }}
                  title={`Full ID: ${space.id}`}
                >
                  📋 Copy ID ({space.id.substring(0, 8)}...)
                </button>
              </div>

              <div className="space-stats">
                <span>👥 {space.members.length} members</span>
                <span>📁 {space.channels.length} channels</span>
                <span>🎭 {space.role_count} roles</span>
              </div>

              <div className="members-list">
                <strong>Members:</strong>
                {space.members.map((member) => (
                  <div key={member.user_id} className="member-item">
                    <span className="member-id">{member.user_id}</span>
                    <span className="member-role">{member.role}</span>
                  </div>
                ))}
              </div>

              {space.channels.length > 0 && (
                <div className="channels-list">
                  <strong>Channels:</strong>
                  {space.channels.map((channel) => (
                    <div key={channel.id} className="channel-item">
                      <span>#{channel.name}</span>
                      <span className="msg-count">
                        {channel.message_count} msgs
                      </span>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
