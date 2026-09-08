import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

import {
  SELECTED_TRANSPORT_MODULE_ID,
  TARGET_ENV_VAR,
  parseTarget,
  transportModuleForTarget,
} from "./src/graphql/transport/targetSelection.ts";
import {
  DEVELOPMENT_SERVER_PROCESS_ORIGIN,
  HTTP_ENDPOINT,
  WEBSOCKET_ENDPOINT,
} from "./src/graphql/transport/webEndpoints.ts";

// One UI, one Target per bundle: the alias below is the only place the two
// Transports differ, and resolving it here rather than at runtime is what
// keeps each Target's Transport out of the other Target's bundle.
const target = parseTarget(process.env[TARGET_ENV_VAR]);
const transportModule = fileURLToPath(
  new URL(transportModuleForTarget(target), import.meta.url),
);

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  define: { __DESKTOP__: JSON.stringify(target === "desktop") },
  resolve: {
    alias: {
      [SELECTED_TRANSPORT_MODULE_ID]: transportModule,
    },
  },
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    // Web Target development is served from one origin exactly as a deployment
    // is: the bundler answers for the UI and forwards GraphQL traffic to the
    // Server Process, so the client keeps using its relative endpoints and
    // there is no development-only URL to configure. The WebSocket entry is
    // listed first because these keys match by prefix. The Desktop Target
    // reaches Rust over Tauri IPC and needs no proxy at all.
    proxy:
      target === "web"
        ? {
            [WEBSOCKET_ENDPOINT]: {
              target: DEVELOPMENT_SERVER_PROCESS_ORIGIN,
              ws: true,
            },
            [HTTP_ENDPOINT]: { target: DEVELOPMENT_SERVER_PROCESS_ORIGIN },
          }
        : undefined,
  },
  build: {
    outDir: target === "web" ? "dist-web" : "dist",
    target: "es2022",
  },
});
