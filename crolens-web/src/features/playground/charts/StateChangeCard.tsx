import { ArrowRight, AlertTriangle, CheckCircle, MinusCircle, PlusCircle } from 'lucide-react';

type StateChange = {
  description: string;
  from: string;
  to: string;
  amount: string;
  token: string;
};

type StateChangeCardProps = {
  changes: StateChange[];
  success: boolean;
  gasEstimated?: string;
  className?: string;
};

function BalanceChange({ change }: { change: StateChange }) {
  // Determine if this is an outgoing (negative) or incoming (positive) change
  const isOutgoing = change.from.toLowerCase().includes('user') ||
    change.description.toLowerCase().includes('spend') ||
    change.description.toLowerCase().includes('transfer out');

  const isIncoming = change.to.toLowerCase().includes('user') ||
    change.description.toLowerCase().includes('receive') ||
    change.description.toLowerCase().includes('transfer in');

  const changeType = isOutgoing ? 'outgoing' : isIncoming ? 'incoming' : 'neutral';

  const colors = {
    outgoing: {
      bg: 'bg-[#FF4444]/10',
      border: 'border-[#FF4444]',
      text: 'text-[#FF4444]',
      icon: MinusCircle,
    },
    incoming: {
      bg: 'bg-[#00FF41]/10',
      border: 'border-[#00FF41]',
      text: 'text-[#00FF41]',
      icon: PlusCircle,
    },
    neutral: {
      bg: 'bg-[#FFD700]/10',
      border: 'border-[#FFD700]',
      text: 'text-[#FFD700]',
      icon: ArrowRight,
    },
  };

  const style = colors[changeType];
  const Icon = style.icon;

  return (
    <div className={`relative p-4 border-l-4 ${style.border} ${style.bg}`}>
      {/* P5 corner decoration */}
      <div className="absolute top-0 right-0 w-4 h-4 border-t border-r border-white/10" />

      <div className="flex items-start justify-between gap-4">
        <div className="flex-1">
          <div className="flex items-center gap-2 mb-2">
            <Icon size={16} className={style.text} />
            <span className="font-bebas tracking-wider text-white">{change.description}</span>
          </div>

          {/* Amount display */}
          <div className="flex items-center gap-3 mb-2">
            <div className="bg-black/50 px-3 py-1 transform -skew-x-6">
              <span className="skew-x-6 inline-block font-mono text-lg text-white">
                {changeType === 'outgoing' ? '-' : changeType === 'incoming' ? '+' : ''}
                {change.amount}
              </span>
            </div>
            <span className={`font-bebas text-lg ${style.text}`}>{change.token}</span>
          </div>

          {/* From/To addresses */}
          <div className="flex items-center gap-2 text-xs font-mono text-[#555]">
            <span className="truncate max-w-[100px]">{change.from}</span>
            <ArrowRight size={12} className="text-[#333]" />
            <span className="truncate max-w-[100px]">{change.to}</span>
          </div>
        </div>
      </div>
    </div>
  );
}

export function StateChangeCard({ changes, success, gasEstimated, className = '' }: StateChangeCardProps) {
  if (changes.length === 0) {
    return (
      <div className={`flex items-center justify-center h-full text-[#555] font-mono ${className}`}>
        No state changes detected
      </div>
    );
  }

  return (
    <div className={`relative ${className}`}>
      {/* Header */}
      <div className="mb-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="w-4 h-4 bg-[#D90018] transform rotate-45" />
          <span className="font-bebas text-[#A3A3A3] tracking-wider">STATE CHANGES PREVIEW</span>
        </div>

        {/* Status badge */}
        <div className={`flex items-center gap-2 px-3 py-1 ${success ? 'bg-[#00FF41]/10 border border-[#00FF41]/30' : 'bg-[#FF4444]/10 border border-[#FF4444]/30'}`}>
          {success ? (
            <CheckCircle size={14} className="text-[#00FF41]" />
          ) : (
            <AlertTriangle size={14} className="text-[#FF4444]" />
          )}
          <span className={`font-mono text-xs ${success ? 'text-[#00FF41]' : 'text-[#FF4444]'}`}>
            {success ? 'SIMULATION OK' : 'SIMULATION FAILED'}
          </span>
        </div>
      </div>

      {/* Gas estimate */}
      {gasEstimated && (
        <div className="mb-4 flex items-center gap-2 text-xs">
          <span className="text-[#555]">Estimated Gas:</span>
          <span className="font-mono text-[#A3A3A3]">{gasEstimated}</span>
        </div>
      )}

      {/* Changes list */}
      <div className="space-y-3">
        {changes.map((change, index) => (
          <BalanceChange key={index} change={change} />
        ))}
      </div>

      {/* Summary */}
      <div className="mt-4 pt-4 border-t border-[#333]">
        <div className="flex items-center justify-between text-sm">
          <span className="text-[#555]">Total changes</span>
          <span className="font-mono text-white">{changes.length}</span>
        </div>
      </div>

      {/* Warning if simulation shows issues */}
      {!success && (
        <div className="mt-4 p-3 bg-[#FF4444]/10 border border-[#FF4444]/30 flex items-start gap-2">
          <AlertTriangle size={16} className="text-[#FF4444] mt-0.5" />
          <div>
            <div className="font-bebas text-[#FF4444] tracking-wider">WARNING</div>
            <div className="text-xs text-[#A3A3A3]">
              This transaction may fail or produce unexpected results. Review carefully before proceeding.
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
