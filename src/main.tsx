import React from "react";
import ReactDOM from "react-dom/client";
import "./index.css";

function FallbackApp() {
  return (
    <div style={{
      height: "100vh", background: "#08080c", color: "#e4e4e7",
      display: "flex", flexDirection: "column", alignItems: "center",
      justifyContent: "center", fontFamily: "system-ui, sans-serif",
    }}>
      <div style={{ fontSize: 32, fontWeight: 700, marginBottom: 16 }}>
        GAME<span style={{ color: "#6366f1" }}>++</span>
      </div>
      <div id="root-msg" style={{ fontSize: 12, color: "#71717a" }}>
        Loading...
      </div>
    </div>
  );
}

const root = document.getElementById("root");
if (root) {
  try {
    ReactDOM.createRoot(root).render(
      <React.StrictMode>
        <FallbackApp />
      </React.StrictMode>
    );
    const msg = document.getElementById("root-msg");
    if (msg) msg.textContent = "Fallback OK - loading App...";

    // Try to load the real App
    import("./App").then(mod => {
      ReactDOM.createRoot(root).render(
        <React.StrictMode>
          <mod.default />
        </React.StrictMode>
      );
    }).catch(e => {
      if (msg) msg.textContent = "App load error: " + String(e);
    });
  } catch (e) {
    root.innerHTML = '<div style="color:red;padding:40px;font:14px monospace;">CRASH: ' + String(e) + '</div>';
  }
}
