import { useState } from "react";
import "./TutorialPanel.css";

interface Step {
  id: number;
  title: string;
  description: string;
  actions: string[];
  example?: {
    client: string;
    action: string;
    fields: { [key: string]: string };
  };
  note?: string;
}

const TUTORIAL_STEPS: Step[] = [
  {
    id: 1,
    title: "Step 1: Alice Creates a Space",
    description:
      "First, a user (Alice) needs to create a new Space. A Space is like a server or community.",
    actions: [
      "Go to the 'Dashboard' tab",
      "Select 'Alice' from the Client dropdown",
      "Choose action: 'Create Space'",
      "Enter a name (e.g., 'red')",
      "Click 'Execute'",
    ],
    example: {
      client: "Alice",
      action: "CreateSpace",
      fields: {
        "Space Name": "red",
      },
    },
    note: "After creation, you'll see the Space appear in Alice's panel with a Space ID (64 hex characters).",
  },
  {
    id: 2,
    title: "Step 2: Copy the Full Space ID",
    description:
      "Bob needs the complete 64-character Space ID to join. The UI only shows the first 8 characters.",
    actions: [
      "Look at Alice's panel in the Dashboard tab",
      "Find the newly created Space (e.g., 'red')",
      "Click the '📋 Copy ID' button next to the space name",
      "An alert will show confirming the full ID was copied",
    ],
    note: "⚠️ IMPORTANT: You must use the full 64-character Space ID, not just the first 8 characters!",
  },
  {
    id: 3,
    title: "Step 3: Connect Alice and Bob's Nodes (P2P Network)",
    description:
      "Before Bob can join, the P2P nodes need to be connected. This simulates peer discovery in a real network.",
    actions: [
      "In the Dashboard backend terminal, you'll see each client's Peer ID",
      "Note: In this isolated dashboard, nodes are NOT automatically connected",
      "For now, this is a limitation - nodes need to discover each other via DHT or relay",
      "Check the 'Network Topology' panel to see connection status",
    ],
    note: "⚠️ CURRENT LIMITATION: The dashboard clients run in isolation. In a real deployment, nodes would discover each other via Kademlia DHT or relay servers. For the tutorial to work, you may need to implement manual peer connections or wait for DHT discovery (which requires bootstrap nodes).",
  },
  {
    id: 4,
    title: "Step 4: Alice Creates an Invite",
    description:
      "Alice creates an invite code that allows others to join the Space.",
    actions: [
      "Select 'Alice' from the Client dropdown",
      "Choose action: 'Create Invite'",
      "Paste the full Space ID (from Step 2)",
      "Click 'Execute'",
    ],
    example: {
      client: "Alice",
      action: "CreateInvite",
      fields: {
        "Space ID": "eb2798d3bae58d5a... (64 chars)",
      },
    },
    note: "The response will show an 8-character invite code like 'ABC123XY'.",
  },
  {
    id: 5,
    title: "Step 5: Bob Joins the Space",
    description: "Bob uses the full Space ID to join Alice's Space.",
    actions: [
      "Select 'Bob' from the Client dropdown",
      "Choose action: 'Join Space'",
      "Paste the same full Space ID (from Step 2)",
      "Click 'Execute'",
      "If you see an error about 'Space not found', it means the nodes aren't connected yet",
    ],
    example: {
      client: "Bob",
      action: "JoinSpace",
      fields: {
        "Space ID (64 chars)": "eb2798d3bae58d5a... (64 chars)",
      },
    },
    note: "⚠️ Common Error: 'Space not found' means Bob's node hasn't discovered the Space data from Alice yet. This happens because the dashboard nodes are isolated and don't have network connectivity established.",
  },
  {
    id: 6,
    title: "Step 6: Verify Membership",
    description:
      "Check that both Alice and Bob now see each other as members of the Space.",
    actions: [
      "Look at Alice's panel - the Space should show 'Members: 2'",
      "Look at Bob's panel - the same Space should appear with 'Members: 2'",
      "Both users can now create channels and send messages",
    ],
    note: "✅ Success! Bob has joined Alice's Space and they can now collaborate.",
  },
];

export default function TutorialPanel() {
  const [currentStep, setCurrentStep] = useState(0);
  const [showAll, setShowAll] = useState(false);

  const handlePrevious = () => {
    setCurrentStep(Math.max(0, currentStep - 1));
  };

  const handleNext = () => {
    setCurrentStep(Math.min(TUTORIAL_STEPS.length - 1, currentStep + 1));
  };

  if (showAll) {
    return (
      <div className="tutorial-panel">
        <div className="tutorial-header">
          <h3>📚 Complete Tutorial: How to Create & Join a Space</h3>
          <button className="toggle-view-btn" onClick={() => setShowAll(false)}>
            Switch to Step-by-Step View
          </button>
        </div>
        <div className="all-steps">
          {TUTORIAL_STEPS.map((step) => (
            <div key={step.id} className="step-card">
              <div className="step-number">Step {step.id}</div>
              <h4>{step.title}</h4>
              <p className="step-description">{step.description}</p>
              <div className="step-actions">
                <h5>Actions:</h5>
                <ol>
                  {step.actions.map((action, idx) => (
                    <li key={idx}>{action}</li>
                  ))}
                </ol>
              </div>
              {step.example && (
                <div className="step-example">
                  <h5>Example:</h5>
                  <div className="example-box">
                    <div className="example-field">
                      <strong>Client:</strong> {step.example.client}
                    </div>
                    <div className="example-field">
                      <strong>Action:</strong> {step.example.action}
                    </div>
                    {Object.entries(step.example.fields).map(([key, value]) => (
                      <div key={key} className="example-field">
                        <strong>{key}:</strong> <code>{value}</code>
                      </div>
                    ))}
                  </div>
                </div>
              )}
              {step.note && (
                <div className="step-note">
                  <strong>💡 Note:</strong> {step.note}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    );
  }

  const step = TUTORIAL_STEPS[currentStep];

  return (
    <div className="tutorial-panel">
      <div className="tutorial-header">
        <h3>📚 Tutorial: How to Create & Join a Space</h3>
        <button className="toggle-view-btn" onClick={() => setShowAll(true)}>
          View All Steps
        </button>
      </div>

      <div className="step-navigation">
        <button
          className="nav-btn"
          onClick={handlePrevious}
          disabled={currentStep === 0}
        >
          ← Previous
        </button>
        <div className="step-indicator">
          Step {currentStep + 1} of {TUTORIAL_STEPS.length}
        </div>
        <button
          className="nav-btn"
          onClick={handleNext}
          disabled={currentStep === TUTORIAL_STEPS.length - 1}
        >
          Next →
        </button>
      </div>

      <div className="step-content">
        <div className="step-header">
          <div className="step-number-large">{step.id}</div>
          <h2>{step.title}</h2>
        </div>

        <p className="step-description-large">{step.description}</p>

        <div className="step-actions-box">
          <h4>📝 Actions to Take:</h4>
          <ol className="action-list">
            {step.actions.map((action, idx) => (
              <li key={idx}>{action}</li>
            ))}
          </ol>
        </div>

        {step.example && (
          <div className="step-example-large">
            <h4>💻 Example:</h4>
            <div className="example-box-large">
              <div className="example-row">
                <span className="example-label">Client:</span>
                <span className="example-value">{step.example.client}</span>
              </div>
              <div className="example-row">
                <span className="example-label">Action:</span>
                <span className="example-value action-name">
                  {step.example.action}
                </span>
              </div>
              {Object.entries(step.example.fields).map(([key, value]) => (
                <div key={key} className="example-row">
                  <span className="example-label">{key}:</span>
                  <span className="example-value">
                    <code>{value}</code>
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}

        {step.note && (
          <div className="step-note-large">
            <strong>💡 Important Note:</strong>
            <p>{step.note}</p>
          </div>
        )}
      </div>

      <div className="step-progress">
        <div className="progress-bar">
          {TUTORIAL_STEPS.map((_, idx) => (
            <div
              key={idx}
              className={`progress-dot ${idx === currentStep ? "active" : ""} ${
                idx < currentStep ? "completed" : ""
              }`}
              onClick={() => setCurrentStep(idx)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}
