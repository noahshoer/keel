import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { initCore } from "./core";
import * as diag from "./diag";
import "./styles/app.css";

const container = document.getElementById("root");
if (container === null) {
  throw new Error('Keel: index.html has no element with id "root" to mount into.');
}

// Diagnostics first, so a failure loading the wasm is logged when they're on.
await diag.init();
// keel-core (wasm) must be loaded before anything renders that calls it.
await initCore();

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
