import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { createConfig, http } from "wagmi";
import { cronos } from "wagmi/chains";
import { injected } from "wagmi/connectors";

export const wagmiChains = [cronos] as const;

const walletConnectProjectId =
  typeof import.meta.env.VITE_WALLETCONNECT_PROJECT_ID === "string"
    ? import.meta.env.VITE_WALLETCONNECT_PROJECT_ID.trim()
    : "";

export const wagmiConfig =
  walletConnectProjectId.length > 0
    ? getDefaultConfig({
        appName: "CroLens",
        projectId: walletConnectProjectId,
        chains: wagmiChains,
        ssr: false,
      })
    : createConfig({
        chains: wagmiChains,
        connectors: [injected({ shimDisconnect: true })],
        transports: {
          [cronos.id]: http(),
        },
      });
