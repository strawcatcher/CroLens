import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RainbowKitProvider, darkTheme } from "@rainbow-me/rainbowkit";
import { WagmiProvider } from "wagmi";
import { Toaster } from "sonner";
import "@rainbow-me/rainbowkit/styles.css";
import "./index.css";
import App from "@/App";
import { validateEnv } from "@/lib/env";
import { setupMonitoring } from "@/lib/monitoring";
import { wagmiConfig } from "@/lib/wagmi";

document.documentElement.classList.add("dark");

validateEnv();
setupMonitoring();

const queryClient = new QueryClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <WagmiProvider config={wagmiConfig}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider
          theme={darkTheme({
            accentColor: "#ff0000",
            accentColorForeground: "#000000",
            borderRadius: "small",
          })}
        >
          <BrowserRouter>
            <App />
          </BrowserRouter>
          <Toaster
            theme="dark"
            position="top-right"
            closeButton
            duration={3000}
            offset={{ top: 80, right: 24 }}
            toastOptions={{
              classNames: {
                toast: "crolens-toast",
              },
            }}
          />
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  </StrictMode>,
);
