import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

// Placeholder settings panel; provider keys and hotkey settings come next.
function Panel() {
  return (
    <main style={{ fontFamily: "-apple-system, 'Segoe UI', sans-serif", padding: 24 }}>
      <h1 style={{ fontSize: 20, margin: 0 }}>Flitty</h1>
      <p style={{ color: "#555" }}>Hold Ctrl + Alt + Space and ask about anything on your screen.</p>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Panel />
  </StrictMode>,
);
