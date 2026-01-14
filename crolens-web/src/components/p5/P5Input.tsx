type P5InputProps = {
  label?: string;
  value: string;
  onChange: (e: React.ChangeEvent<HTMLInputElement>) => void;
  placeholder?: string;
  type?: string;
  rightElement?: React.ReactNode;
  className?: string;
  id?: string;
  'aria-invalid'?: boolean;
};

export function P5Input({
  label,
  value,
  onChange,
  placeholder,
  type = "text",
  rightElement,
  className = '',
  id,
  'aria-invalid': ariaInvalid,
}: P5InputProps) {
  return (
    <div className={`mb-4 ${className}`}>
      {label && (
        <label
          htmlFor={id}
          className="block font-bebas tracking-wider text-[#A3A3A3] mb-1 ml-1 text-lg"
        >
          {label}
        </label>
      )}
      <div className="relative flex items-center bg-[#242424] border-2 border-[#333] focus-within:border-[#D90018] focus-within:shadow-[0_0_10px_rgba(217,0,24,0.3)] transition-all rounded-sm overflow-hidden">
        <input
          id={id}
          type={type}
          value={value}
          onChange={onChange}
          placeholder={placeholder}
          aria-invalid={ariaInvalid}
          className="w-full bg-transparent border-none text-white font-mono px-4 py-3 outline-none placeholder-[#555] h-12"
          spellCheck="false"
        />
        {rightElement && (
          <div className="border-l border-[#333] h-12 flex items-center px-3 bg-[#1e1e1e]">
            {rightElement}
          </div>
        )}
      </div>
    </div>
  );
}
