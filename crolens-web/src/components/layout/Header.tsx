import { useState } from 'react';
import { NavLink } from 'react-router-dom';
import { ConnectButton } from '@rainbow-me/rainbowkit';
import { Terminal, Activity, Box, Zap, X, Menu } from 'lucide-react';
import { cn } from '@/lib/utils';
import { P5Button } from '@/components/p5';

type NavItemProps = {
  to: string;
  label: string;
  icon: React.ComponentType<{ size?: number | string }>;
};

function NavItem({ to, label, icon: Icon }: NavItemProps) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          "group relative h-full px-8 flex items-center justify-center transition-all",
          isActive ? "text-black" : "text-[#A3A3A3] hover:text-white"
        )
      }
    >
      {({ isActive }) => (
        <>
          {/* 选中背景 (斜切) */}
          {isActive && (
            <div className="absolute inset-y-0 inset-x-2 bg-[#D90018] transform -skew-x-12 z-0" />
          )}

          {/* 悬停下划线 (未选中) */}
          {!isActive && (
            <div className="absolute bottom-0 left-4 right-4 h-[2px] bg-[#D90018] transform scale-x-0 group-hover:scale-x-100 transition-transform" />
          )}

          <span className="relative z-10 font-bebas text-xl tracking-wider flex items-center gap-2">
            <Icon size={16} /> {label}
          </span>
        </>
      )}
    </NavLink>
  );
}

function WalletButton() {
  return (
    <ConnectButton.Custom>
      {({
        account,
        chain,
        openAccountModal,
        openConnectModal,
        openChainModal,
        mounted,
      }) => {
        const ready = mounted;
        const connected = ready && account && chain;

        if (!ready) {
          return (
            <div
              className="h-10 w-[140px] rounded-sm border border-[#333] bg-[#1A1A1A]"
              aria-hidden="true"
            />
          );
        }

        if (!connected) {
          return (
            <P5Button variant="secondary" onClick={openConnectModal}>
              CONNECT
            </P5Button>
          );
        }

        if (chain.unsupported) {
          return (
            <P5Button variant="secondary" onClick={openChainModal}>
              WRONG NETWORK
            </P5Button>
          );
        }

        return (
          <P5Button variant="secondary" onClick={openAccountModal}>
            {account.displayName}
          </P5Button>
        );
      }}
    </ConnectButton.Custom>
  );
}

export function Header() {
  const [isMenuOpen, setIsMenuOpen] = useState(false);

  const navItems: NavItemProps[] = [
    { to: '/playground', label: 'PLAYGROUND', icon: Terminal },
    { to: '/dashboard', label: 'DASHBOARD', icon: Activity },
    { to: '/console', label: 'CONSOLE', icon: Box },
  ];

  return (
    <>
      <header className="fixed top-0 left-0 right-0 z-50 bg-[#0A0A0A] border-b border-[#333]">
        <div className="max-w-[1600px] mx-auto px-4 h-16 flex items-center justify-between">
          {/* Logo */}
          <NavLink to="/playground" className="flex items-center gap-3" aria-label="Go to Playground">
            <div className="w-8 h-8 bg-[#D90018] transform rotate-45 flex items-center justify-center shadow-[0_0_10px_rgba(217,0,24,0.5)]">
              <Zap className="transform -rotate-45 text-black" fill="black" size={18} />
            </div>
            <span className="font-bebas text-2xl tracking-widest text-white mt-1">
              CROLENS <span className="text-[#D90018]">MCP</span>
            </span>
          </NavLink>

          {/* Desktop Nav */}
          <nav className="hidden md:flex items-center h-full" aria-label="Primary">
            {navItems.map((item) => (
              <NavItem key={item.to} {...item} />
            ))}
          </nav>

          {/* Wallet Button (Desktop) */}
          <div className="hidden md:block">
            <WalletButton />
          </div>

          {/* Mobile Menu Toggle */}
          <button
            className="md:hidden text-white p-2"
            onClick={() => setIsMenuOpen(!isMenuOpen)}
            aria-label={isMenuOpen ? "Close menu" : "Open menu"}
          >
            {isMenuOpen ? <X size={24} /> : <Menu size={24} />}
          </button>
        </div>
      </header>

      {/* Mobile Menu */}
      {isMenuOpen && (
        <div className="fixed inset-0 z-40 bg-black/95 pt-20 px-6 md:hidden">
          <div className="flex flex-col gap-4">
            {navItems.map((item) => (
              <NavLink
                key={item.to}
                to={item.to}
                onClick={() => setIsMenuOpen(false)}
                className={({ isActive }) =>
                  cn(
                    "w-full py-4 font-bebas text-2xl tracking-widest text-left border-b border-[#333]",
                    isActive ? "text-[#D90018]" : "text-white"
                  )
                }
              >
                {item.label}
              </NavLink>
            ))}
            <div className="pt-4">
              <WalletButton />
            </div>
          </div>
        </div>
      )}

      {/* Spacer for fixed header */}
      <div className="h-16" />
    </>
  );
}
