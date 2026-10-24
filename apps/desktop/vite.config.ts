import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// The interface is served by the app itself; nothing is loaded from the network.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { host: "localhost", port: 5173, strictPort: true },
  build: {
    outDir: "dist",
    target: ["es2022", "chrome120", "safari16"],
    // every asset is a file of the app: nothing is inlined as a data: address (content security policy)
    assetsInlineLimit: 0,
    sourcemap: false,
  },
});
