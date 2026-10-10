import type { ReactNode } from "react";
import { cn } from "@/lib/utils";

export function EmptyState({
  icon,
  title,
  description,
  action,
  className,
}: {
  icon: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
  className?: string;
}) {
  return (
    <div className={cn("flex animate-rise flex-col items-center justify-center px-8 py-20 text-center", className)}>
      <div className="relative mb-6">
        <div className="absolute inset-0 rounded-full bg-accent/20 blur-2xl" />
        <div className="relative grid size-20 place-items-center rounded-full bg-gradient-to-br from-forest to-surface-2 text-accent ring-1 ring-accent/20 [&_svg]:size-9">
          {icon}
        </div>
      </div>
      <h2 className="font-display text-2xl font-bold tracking-tight">{title}</h2>
      {description && <p className="mt-2 max-w-md text-[15px] text-balance text-muted">{description}</p>}
      {action && <div className="mt-7 flex flex-wrap justify-center gap-3">{action}</div>}
    </div>
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
  children,
}: {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
  children?: ReactNode;
}) {
  return (
    <header className="px-8 pt-24 pb-6">
      <div className="flex flex-wrap items-end justify-between gap-4">
        <div className="min-w-0">
          <h1 className="font-display text-[34px] leading-tight font-extrabold tracking-tight">{title}</h1>
          {subtitle && <p className="mt-1 text-sm text-muted">{subtitle}</p>}
        </div>
        {actions && <div className="flex flex-wrap items-center gap-2">{actions}</div>}
      </div>
      {children}
    </header>
  );
}

/** Pill-style segmented control used for tabs and sort options. */
export function Segmented<T extends string>({
  value,
  options,
  onChange,
  className,
}: {
  value: T;
  options: { value: T; label: string }[];
  onChange: (v: T) => void;
  className?: string;
}) {
  return (
    <div className={cn("flex flex-wrap gap-2", className)}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          onClick={() => onChange(o.value)}
          className={cn(
            "h-8 rounded-full px-3.5 text-[13px] font-medium transition-colors",
            value === o.value ? "bg-fg text-bg" : "bg-white/[0.07] text-fg hover:bg-white/[0.12]",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}
