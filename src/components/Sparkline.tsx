import { useId, useState } from "react";

/**
 * A tiny line of recent readings with the current value beside it. One series, so no
 * legend; the label and value are text, so the line's color never carries meaning alone.
 * Hovering shows the reading under the pointer.
 */
export default function Sparkline({
  label,
  values,
  format,
  /** Lowest top of the scale, so an idle service's line stays flat instead of magnifying noise. */
  floor,
  note,
}: {
  label: string;
  /** Extra tooltip text, e.g. what 100% means. */
  note?: string;
  values: number[];
  format: (v: number) => string;
  floor: number;
}) {
  const id = useId();
  const [hover, setHover] = useState<number | null>(null);
  const w = 72;
  const h = 22;
  const max = Math.max(floor, ...values);
  const step = values.length > 1 ? w / (values.length - 1) : w;
  const y = (v: number) => h - 2 - (v / max) * (h - 4);
  const points = values.map((v, i) => `${(i * step).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  const current = values.length ? values[values.length - 1] : undefined;
  const shown = hover !== null ? values[hover] : current;

  return (
    <div className="flex min-w-0 flex-col gap-0.5">
      <div className="flex gap-0.5">
        <span className="text-xs font-medium text-muted">{label}</span>
        <span className="font-mono text-[11px] text-fg tabular-nums ml-auto">{shown === undefined ? "—" : format(shown)}</span>
      </div>
      <div className="border-slate-200 border">
        <svg
          width={w}
          height={h}
          viewBox={`0 0 ${w} ${h}`}
          role="img"
          aria-labelledby={id}
          className="shrink-0 overflow-visible"
          onMouseMove={(e) => {
            if (values.length < 2) return;
            const x = e.clientX - e.currentTarget.getBoundingClientRect().left;
            setHover(Math.max(0, Math.min(values.length - 1, Math.round(x / step))));
          }}
          onMouseLeave={() => setHover(null)}
        >
          <title id={id}>
            {current === undefined
              ? `${label}: no readings yet`
              : `${label} ${format(current)}, last minute peak ${format(Math.max(...values))}${note ? `. ${note}` : ""}`}
          </title>
          {/* recessive baseline */}
          <line x1={0} x2={w} y1={h - 2} y2={h - 2} className="stroke-line" strokeWidth={1} />
          {values.length > 1 && (
            <polyline points={points} fill="none" className="stroke-accent" strokeWidth={1.75} strokeLinejoin="round" strokeLinecap="round" />
          )}
          {hover !== null && values.length > 1 && (
            <>
              <line x1={hover * step} x2={hover * step} y1={0} y2={h} className="stroke-muted/40" strokeWidth={1} />
              <circle cx={hover * step} cy={y(values[hover])} r={2.5} className="fill-accent stroke-surface" strokeWidth={1.5} />
            </>
          )}
        </svg>
      </div>
    </div>
  );
}
