import type { NetworkNode, NetworkEdge } from "../App";
import "./NetworkGraph.css";

interface NetworkGraphProps {
  nodes: NetworkNode[];
  edges: NetworkEdge[];
}

export default function NetworkGraph({ nodes, edges }: NetworkGraphProps) {
  if (nodes.length === 0) {
    return <div className="network-empty">No network connections yet</div>;
  }

  return (
    <div className="network-graph">
      <svg width="100%" height="350" viewBox="0 0 400 350">
        {/* Draw edges */}
        {edges.map((edge, idx) => {
          const fromNode = nodes.find((n) => n.id === edge.from);
          const toNode = nodes.find((n) => n.id === edge.to);

          if (!fromNode || !toNode) return null;

          const fromIdx = nodes.indexOf(fromNode);
          const toIdx = nodes.indexOf(toNode);

          // Arrange nodes in a circle
          const centerX = 200;
          const centerY = 175;
          const radius = 120;

          const fromAngle = (fromIdx / nodes.length) * 2 * Math.PI;
          const toAngle = (toIdx / nodes.length) * 2 * Math.PI;

          const x1 = centerX + radius * Math.cos(fromAngle);
          const y1 = centerY + radius * Math.sin(fromAngle);
          const x2 = centerX + radius * Math.cos(toAngle);
          const y2 = centerY + radius * Math.sin(toAngle);

          return (
            <line
              key={idx}
              x1={x1}
              y1={y1}
              x2={x2}
              y2={y2}
              stroke="#30363d"
              strokeWidth="2"
              strokeDasharray={edge.edge_type === "dht" ? "5,5" : "0"}
            />
          );
        })}

        {/* Draw nodes */}
        {nodes.map((node, idx) => {
          const centerX = 200;
          const centerY = 175;
          const radius = 120;
          const angle = (idx / nodes.length) * 2 * Math.PI;

          const x = centerX + radius * Math.cos(angle);
          const y = centerY + radius * Math.sin(angle);

          return (
            <g key={node.id}>
              <circle
                cx={x}
                cy={y}
                r="30"
                fill="#161b22"
                stroke="#58a6ff"
                strokeWidth="2"
              />
              <text
                x={x}
                y={y}
                textAnchor="middle"
                dominantBaseline="middle"
                fill="#c9d1d9"
                fontSize="14"
                fontWeight="600"
              >
                {node.label}
              </text>
              <text
                x={x}
                y={y + 50}
                textAnchor="middle"
                fill="#8b949e"
                fontSize="10"
              >
                {node.id.substring(0, 6)}
              </text>
            </g>
          );
        })}
      </svg>

      <div className="legend">
        <div className="legend-item">
          <div className="legend-line solid"></div>
          <span>GossipSub</span>
        </div>
        <div className="legend-item">
          <div className="legend-line dashed"></div>
          <span>DHT</span>
        </div>
      </div>
    </div>
  );
}
