import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-react": ["react", "react-dom", "react-router-dom"],
          "vendor-ui": ["framer-motion", "lucide-react", "sonner", "clsx"],
          "vendor-query": ["@tanstack/react-query"],
          "vendor-web3": ["viem", "wagmi"],
          "vendor-rainbowkit": ["@rainbow-me/rainbowkit"],
          "vendor-chart": ["recharts"],
          "vendor-syntax": ["react-syntax-highlighter"],
        },
      },
    },
  },
});
