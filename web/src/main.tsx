import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { initCore } from "./core";

const container = document.getElementById("root");
if (container === null) {
  throw new Error('Keel: index.html has no element with id "root" to mount into.');
}

// keel-core (wasm) must be loaded before anything renders that calls it.
await initCore();

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
