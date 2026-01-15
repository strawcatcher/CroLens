export function PageSkeleton() {
  return (
    <div className="animate-pulse space-y-6">
      {/* Header skeleton */}
      <div className="flex items-center justify-between">
        <div className="h-8 w-48 bg-[#1A1A1A] rounded" />
        <div className="h-10 w-32 bg-[#1A1A1A] rounded" />
      </div>

      {/* Main content skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        {/* Left panel */}
        <div className="lg:col-span-2 space-y-4">
          <div className="h-12 bg-[#1A1A1A] rounded" />
          <div className="h-64 bg-[#1A1A1A] rounded" />
        </div>

        {/* Right panel */}
        <div className="space-y-4">
          <div className="h-32 bg-[#1A1A1A] rounded" />
          <div className="h-48 bg-[#1A1A1A] rounded" />
        </div>
      </div>
    </div>
  );
}
