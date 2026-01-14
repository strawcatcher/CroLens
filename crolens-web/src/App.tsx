import { Navigate, Route, Routes, useLocation } from "react-router-dom";
import { AnimatePresence, motion, useReducedMotion } from "framer-motion";
import { Zap, Github, Twitter, ExternalLink } from "lucide-react";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Header } from "@/components/layout/Header";
import { ConsolePage } from "@/features/console/ConsolePage";
import { DashboardPage } from "@/features/dashboard/DashboardPage";
import { PlaygroundPage } from "@/features/playground/PlaygroundPage";

function Footer() {
  return (
    <footer className="border-t border-[#333] bg-[#0A0A0A]/95 mt-auto">
      <div className="max-w-[1600px] mx-auto px-4 py-8">
        <div className="grid grid-cols-1 md:grid-cols-4 gap-8">
          {/* Logo & Description */}
          <div className="md:col-span-2">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-6 h-6 bg-[#D90018] transform rotate-45 flex items-center justify-center">
                <Zap className="transform -rotate-45 text-black" fill="black" size={14} />
              </div>
              <span className="font-bebas text-xl tracking-widest text-white">
                CROLENS <span className="text-[#D90018]">MCP</span>
              </span>
            </div>
            <p className="text-sm text-[#A3A3A3] font-inter max-w-md">
              AI-native infrastructure layer for the Cronos blockchain.
              Model Context Protocol (MCP) server providing real-time access
              to on-chain data with x402 payment integration.
            </p>
          </div>

          {/* Quick Links */}
          <div>
            <h3 className="font-bebas text-lg tracking-wider text-white mb-4">
              RESOURCES
            </h3>
            <ul className="space-y-2 text-sm">
              <li>
                <a
                  href="https://github.com/anthropics/claude-mcp"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[#A3A3A3] hover:text-[#D90018] transition-colors flex items-center gap-2"
                >
                  <ExternalLink size={12} />
                  MCP Documentation
                </a>
              </li>
              <li>
                <a
                  href="https://cronos.org"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[#A3A3A3] hover:text-[#D90018] transition-colors flex items-center gap-2"
                >
                  <ExternalLink size={12} />
                  Cronos Chain
                </a>
              </li>
              <li>
                <a
                  href="https://cronoscan.com"
                  target="_blank"
                  rel="noopener noreferrer"
                  className="text-[#A3A3A3] hover:text-[#D90018] transition-colors flex items-center gap-2"
                >
                  <ExternalLink size={12} />
                  Cronos Explorer
                </a>
              </li>
            </ul>
          </div>

          {/* Social */}
          <div>
            <h3 className="font-bebas text-lg tracking-wider text-white mb-4">
              CONNECT
            </h3>
            <div className="flex gap-3">
              <a
                href="https://github.com"
                target="_blank"
                rel="noopener noreferrer"
                className="w-10 h-10 bg-[#1A1A1A] border border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                aria-label="GitHub"
              >
                <Github size={18} />
              </a>
              <a
                href="https://twitter.com"
                target="_blank"
                rel="noopener noreferrer"
                className="w-10 h-10 bg-[#1A1A1A] border border-[#333] flex items-center justify-center text-[#A3A3A3] hover:text-white hover:border-[#D90018] transition-colors"
                aria-label="Twitter"
              >
                <Twitter size={18} />
              </a>
            </div>
          </div>
        </div>

        {/* Bottom Bar */}
        <div className="mt-8 pt-6 border-t border-[#333] flex flex-wrap items-center justify-between gap-4">
          <div className="text-xs text-[#555] font-mono">
            © 2025 CROLENS. Built for Cronos Hackathon.
          </div>
          <div className="flex items-center gap-4 text-xs text-[#555] font-mono">
            <span className="flex items-center gap-2">
              <span className="w-2 h-2 rounded-full bg-[#00FF41] animate-pulse" />
              MAINNET LIVE
            </span>
            <span>v1.0.0</span>
          </div>
        </div>
      </div>
    </footer>
  );
}

export default function App() {
  const location = useLocation();
  const reducedMotion = useReducedMotion();

  return (
    <ErrorBoundary title="CroLens crashed">
      <div className="min-h-screen flex flex-col p5-stripe-bg text-foreground">
        <Header />
        <main className="flex-1 container py-8">
          <AnimatePresence mode="wait" initial={false}>
            <motion.div
              key={location.pathname}
              initial={reducedMotion ? false : { opacity: 0, y: 8 }}
              animate={{
                opacity: 1,
                y: 0,
                transition: reducedMotion
                  ? { duration: 0 }
                  : { duration: 0.3, ease: [0, 0, 0.2, 1], delay: 0.05 },
              }}
              exit={{
                opacity: 0,
                transition: reducedMotion
                  ? { duration: 0 }
                  : { duration: 0.15, ease: [0.4, 0, 1, 1] },
              }}
            >
              <Routes location={location}>
                <Route path="/" element={<Navigate to="/playground" replace />} />
                <Route
                  path="/playground"
                  element={
                    <ErrorBoundary title="Playground crashed">
                      <PlaygroundPage />
                    </ErrorBoundary>
                  }
                />
                <Route
                  path="/dashboard"
                  element={
                    <ErrorBoundary title="Dashboard crashed">
                      <DashboardPage />
                    </ErrorBoundary>
                  }
                />
                <Route
                  path="/console"
                  element={
                    <ErrorBoundary title="Console crashed">
                      <ConsolePage />
                    </ErrorBoundary>
                  }
                />
                <Route path="*" element={<Navigate to="/playground" replace />} />
              </Routes>
            </motion.div>
          </AnimatePresence>
        </main>
        <Footer />
      </div>
    </ErrorBoundary>
  );
}
