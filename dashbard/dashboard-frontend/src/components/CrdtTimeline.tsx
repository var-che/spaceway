import type { CrdtOperation } from "../App";
import "./CrdtTimeline.css";

interface CrdtTimelineProps {
  operations: CrdtOperation[];
}

export default function CrdtTimeline({ operations }: CrdtTimelineProps) {
  if (operations.length === 0) {
    return (
      <div className="timeline-empty">
        No CRDT operations yet. Create a space to get started!
      </div>
    );
  }

  return (
    <div className="crdt-timeline">
      {operations.map((op) => (
        <div key={op.op_id} className="timeline-item">
          <div className="timeline-marker"></div>
          <div className="timeline-content">
            <div className="op-header">
              <span className="op-type">{op.op_type}</span>
              <span className="op-time">
                {new Date(op.timestamp * 1000).toLocaleTimeString()}
              </span>
            </div>
            <div className="op-details">
              <span>Author: {op.author.substring(0, 8)}</span>
              <span>Space: {op.space_id.substring(0, 8)}</span>
              {op.channel_id && (
                <span>Channel: {op.channel_id.substring(0, 8)}</span>
              )}
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
