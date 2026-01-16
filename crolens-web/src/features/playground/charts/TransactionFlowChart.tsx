import { ArrowRight, ArrowDown } from 'lucide-react';

type FlowNode = {
  label: string;
  address?: string;
  isUser?: boolean;
  isContract?: boolean;
};

type FlowStep = {
  from: FlowNode;
  to: FlowNode;
  amount: string;
  token: string;
  direction?: 'right' | 'down';
};

type TransactionFlowChartProps = {
  steps: FlowStep[];
  className?: string;
};

function AddressBox({ node, highlight = false }: { node: FlowNode; highlight?: boolean }) {
  const bgColor = node.isUser
    ? 'bg-[#D90018]/20 border-[#D90018]'
    : node.isContract
      ? 'bg-[#00FF41]/10 border-[#00FF41]'
      : 'bg-[#1A1A1A] border-[#333]';

  const textColor = node.isUser
    ? 'text-[#D90018]'
    : node.isContract
      ? 'text-[#00FF41]'
      : 'text-white';

  return (
    <div
      className={`relative px-4 py-3 border-l-4 ${bgColor} ${highlight ? 'shadow-[0_0_20px_rgba(217,0,24,0.3)]' : ''}`}
    >
      {/* P5 corner decoration */}
      <div className="absolute top-0 right-0 w-3 h-3 border-t border-r border-white/20" />

      <div className={`font-bebas tracking-wider ${textColor}`}>
        {node.label}
      </div>
      {node.address && (
        <div className="font-mono text-xs text-[#555] truncate max-w-[120px]">
          {node.address.slice(0, 6)}...{node.address.slice(-4)}
        </div>
      )}
    </div>
  );
}

function FlowArrow({ amount, token, direction = 'right' }: { amount: string; token: string; direction?: 'right' | 'down' }) {
  const isVertical = direction === 'down';

  return (
    <div className={`flex items-center justify-center ${isVertical ? 'flex-col py-2' : 'px-2'}`}>
      {/* Amount badge */}
      <div className="bg-black border border-[#D90018] px-3 py-1 transform -skew-x-6">
        <span className="skew-x-6 inline-block font-mono text-xs text-white">
          {amount} <span className="text-[#D90018]">{token}</span>
        </span>
      </div>

      {/* Arrow */}
      <div className={`${isVertical ? 'mt-1' : 'ml-1'}`}>
        {isVertical ? (
          <ArrowDown className="text-[#D90018]" size={20} />
        ) : (
          <ArrowRight className="text-[#D90018]" size={20} />
        )}
      </div>
    </div>
  );
}

export function TransactionFlowChart({ steps, className = '' }: TransactionFlowChartProps) {
  if (steps.length === 0) {
    return (
      <div className={`flex items-center justify-center h-full text-[#555] font-mono ${className}`}>
        No flow data available
      </div>
    );
  }

  return (
    <div className={`relative ${className}`}>
      {/* P5 decorative header */}
      <div className="mb-4 flex items-center gap-2">
        <div className="w-4 h-4 bg-[#D90018] transform rotate-45" />
        <span className="font-bebas text-[#A3A3A3] tracking-wider">TRANSACTION FLOW</span>
      </div>

      {/* Flow visualization */}
      <div className="space-y-4">
        {steps.map((step, index) => (
          <div key={index} className="flex items-center gap-2 flex-wrap">
            <AddressBox node={step.from} highlight={step.from.isUser} />
            <FlowArrow amount={step.amount} token={step.token} direction={step.direction} />
            <AddressBox node={step.to} highlight={step.to.isUser} />
          </div>
        ))}
      </div>

      {/* Legend */}
      <div className="mt-6 flex items-center gap-4 text-xs">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-[#D90018]/20 border-l-2 border-[#D90018]" />
          <span className="text-[#A3A3A3]">User</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-[#00FF41]/10 border-l-2 border-[#00FF41]" />
          <span className="text-[#A3A3A3]">Contract</span>
        </div>
      </div>
    </div>
  );
}

// Helper function to convert decoded transaction to flow steps
export function createFlowStepsFromTx(decoded: {
  from: string;
  to: string;
  action: string;
  decoded: {
    method_name: string;
    inputs?: Array<{ name: string; value: string }>;
  };
}): FlowStep[] {
  const steps: FlowStep[] = [];

  // Basic transaction flow
  const isSwap = decoded.action.toLowerCase().includes('swap');
  const isTransfer = decoded.action.toLowerCase().includes('transfer');

  if (isSwap) {
    // Parse swap inputs if available
    const amountIn = decoded.decoded.inputs?.find(i => i.name.includes('amountIn'))?.value || '?';
    const amountOut = decoded.decoded.inputs?.find(i => i.name.includes('amountOut'))?.value || '?';

    steps.push({
      from: { label: 'USER', address: decoded.from, isUser: true },
      to: { label: 'ROUTER', address: decoded.to, isContract: true },
      amount: amountIn,
      token: 'IN',
    });

    steps.push({
      from: { label: 'ROUTER', address: decoded.to, isContract: true },
      to: { label: 'USER', address: decoded.from, isUser: true },
      amount: amountOut,
      token: 'OUT',
    });
  } else if (isTransfer) {
    const toAddress = decoded.decoded.inputs?.find(i => i.name === 'to' || i.name === 'recipient')?.value;
    const amount = decoded.decoded.inputs?.find(i => i.name === 'amount' || i.name === 'value')?.value || '?';

    steps.push({
      from: { label: 'FROM', address: decoded.from, isUser: true },
      to: { label: 'TO', address: toAddress || decoded.to, isContract: !toAddress },
      amount: amount,
      token: 'TOKEN',
    });
  } else {
    // Generic call
    steps.push({
      from: { label: 'CALLER', address: decoded.from, isUser: true },
      to: { label: decoded.decoded.method_name.toUpperCase(), address: decoded.to, isContract: true },
      amount: '—',
      token: 'CALL',
    });
  }

  return steps;
}
