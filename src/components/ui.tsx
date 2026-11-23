import type { ComponentProps, ReactNode } from "react";
import { Loader2 } from "lucide-react";
import clsx from "clsx";

type Variant = "primary" | "secondary" | "soft" | "danger" | "ghost";

const variants: Record<Variant, string> = {
  primary: "bg-accent text-accent-fg shadow-card hover:bg-accent-hover",
  secondary: "border border-line bg-surface text-fg shadow-card hover:bg-surface-2",
  // Row actions (Start next to Stop): quiet like secondary, so the accent stays for the one primary action.
  soft: "border border-line bg-surface text-fg shadow-card hover:bg-surface-2",
  danger: "bg-danger text-white hover:bg-danger-hover",
  ghost: "text-muted hover:bg-surface-2 hover:text-fg",
};

const sizes = {
  sm: "h-8 gap-1.5 rounded-md px-2.5 text-sm",
  md: "h-9 gap-2 rounded-lg px-3.5 text-sm",
  lg: "h-11 gap-2 rounded-xl px-5 text-[15px]",
};

export function Button({
  variant = "secondary",
  size = "md",
  className,
  ...props
}: ComponentProps<"button"> & { variant?: Variant; size?: keyof typeof sizes }) {
  return (
    <button
      {...props}
      className={clsx(
        "motion-press inline-flex shrink-0 items-center justify-center font-medium whitespace-nowrap disabled:cursor-not-allowed disabled:opacity-50",
        // A disabled secondary button otherwise looks almost the same as an enabled one in dark mode.
        variant === "secondary" && "disabled:border-transparent disabled:bg-transparent disabled:text-muted disabled:shadow-none",
        sizes[size],
        variants[variant],
        className,
      )}
    />
  );
}

export function PageHeader({
  title,
  subtitle,
  actions,
}: {
  title: string;
  subtitle?: ReactNode;
  actions?: ReactNode;
}) {
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
      <div>
        <h1 className="text-2xl font-medium tracking-tight">{title}</h1>
        {subtitle && <p className="mt-1 text-muted">{subtitle}</p>}
      </div>
      {actions && <div className="flex flex-wrap gap-2">{actions}</div>}
    </header>
  );
}

export function Card({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div className={clsx("rounded-2xl border border-line bg-surface shadow-card", className)}>{children}</div>
  );
}

export function Pill({ tone, children }: { tone: "ok" | "warn" | "muted" | "danger"; children: ReactNode }) {
  return (
    <span
      className={clsx(
        // The status light: a dot carries the colour, the word stays quiet.
        "inline-flex items-center gap-1.5 text-xs font-medium",
        tone === "danger" ? "text-danger-text" : tone === "muted" ? "text-muted" : "text-fg",
      )}
    >
      <span
        className={clsx(
          "size-1.5 rounded-full",
          tone === "ok" && "bg-ok",
          tone === "warn" && "bg-warn",
          tone === "muted" && "bg-muted/60",
          tone === "danger" && "bg-danger",
        )}
      />
      {children}
    </span>
  );
}

export function Switch({
  checked,
  onChange,
  label,
  busy = false,
  disabled = false,
}: {
  checked: boolean;
  onChange: (v: boolean) => void;
  label: string;
  /** Shows a spinner in the knob while the change is being applied. */
  busy?: boolean;
  disabled?: boolean;
}) {
  return (
    <button
      role="switch"
      aria-checked={checked}
      aria-label={label}
      aria-busy={busy || undefined}
      disabled={disabled || busy}
      onClick={() => onChange(!checked)}
      className={clsx(
        "relative h-6 w-11 shrink-0 rounded-full transition-colors disabled:cursor-not-allowed",
        checked ? "bg-accent" : "bg-muted/40",
        disabled && !busy && "opacity-50",
      )}
    >
      <span
        className={clsx(
          "absolute top-0.5 left-0.5 flex size-5 items-center justify-center rounded-full bg-white shadow transition-transform duration-300 ease-(--ease-spring)",
          checked && "translate-x-5",
        )}
      >
        {busy && <Loader2 size={13} strokeWidth={2.5} className="animate-spin text-accent" />}
      </span>
    </button>
  );
}

/** Joined toggle buttons; one option is always selected. */
export function Segmented<T extends string>({
  value,
  onChange,
  options,
  className,
}: {
  value: T;
  onChange: (v: T) => void;
  options: { value: T; label: string; icon?: ReactNode }[];
  className?: string;
}) {
  return (
    <div role="radiogroup" className={clsx("flex rounded-xl border border-line bg-surface p-1 shadow-card", className)}>
      {options.map((o) => (
        <button
          key={o.value}
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className={clsx(
            "flex flex-1 items-center justify-center gap-2 rounded-lg px-3 py-1.5 text-sm font-medium whitespace-nowrap transition-colors",
            value === o.value ? "bg-surface-2 text-fg shadow-card" : "text-muted hover:text-fg",
          )}
        >
          {o.icon}
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function IconTile({ className, children }: { className?: string; children: ReactNode }) {
  return (
    <div className={clsx("flex size-12 shrink-0 items-center justify-center rounded-2xl", className)}>{children}</div>
  );
}

export function SectionCard({
  icon,
  title,
  subtitle,
  actions,
  className,
  children,
}: {
  icon: ReactNode;
  title: string;
  subtitle?: string;
  actions?: ReactNode;
  className?: string;
  children: ReactNode;
}) {
  return (
    <Card className={clsx("p-6", className)}>
      <div className="mb-5 flex items-start gap-4">
        <IconTile className="bg-surface-2 text-muted">{icon}</IconTile>
        <div className="min-w-0 flex-1">
          <h2 className="text-lg font-medium tracking-tight">{title}</h2>
          {subtitle && <p className="text-sm text-muted">{subtitle}</p>}
        </div>
        {actions}
      </div>
      {children}
    </Card>
  );
}

export const inputClass =
  "w-full rounded-lg border border-line bg-bg px-3 py-2 text-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/25";
