// The line icons the shell and Today use, as JSX ports of
// crates/ui/src/icons.rs (same paths, sizes and strokes). Each renders inside
// a `span.icon`, like the Dioxus `Icon` component.
import type { ReactNode } from "react";

function Line({ size, stroke, children }: { size: number; stroke: string; children: ReactNode }) {
  return (
    <span className="icon">
      <svg
        width={size}
        height={size}
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth={stroke}
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
      >
        {children}
      </svg>
    </span>
  );
}

export function InboxIcon() {
  return (
    <Line size={20} stroke="1.6">
      <path d="M4 13h4l1.5 2.5h5L16 13h4" />
      <path d="M5.5 5h13L20 13v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-5z" />
    </Line>
  );
}

/** Calendar showing today's day-of-month. */
export function TodayIcon({ day }: { day: number }) {
  return (
    <Line size={20} stroke="1.6">
      <rect x="4" y="5" width="16" height="15" rx="2" />
      <path d="M4 9h16" />
      <text
        x="12"
        y="17.6"
        textAnchor="middle"
        fontSize="7.5"
        fontWeight="600"
        fill="currentColor"
        stroke="none"
        fontFamily="sans-serif"
      >
        {day}
      </text>
    </Line>
  );
}

export function UpcomingIcon() {
  return (
    <Line size={20} stroke="1.6">
      <rect x="4" y="5" width="16" height="15" rx="2" />
      <path d="M4 9h16M8 3v4M16 3v4M8 13h2M12 13h2M8 16.5h2M12 16.5h2" />
    </Line>
  );
}

export function SearchIcon() {
  return (
    <Line size={20} stroke="1.7">
      <circle cx="11" cy="11" r="6.5" />
      <path d="M16 16l4.5 4.5" />
    </Line>
  );
}

export function MenuIcon() {
  return (
    <Line size={20} stroke="1.8">
      <path d="M4 7h16M4 12h16M4 17h16" />
    </Line>
  );
}

export function MonitorIcon() {
  return (
    <Line size={18} stroke="1.6">
      <rect x="3" y="4" width="18" height="12" rx="2" />
      <path d="M8 20h8M12 16v4" />
    </Line>
  );
}

export function GearIcon() {
  return (
    <Line size={18} stroke="1.6">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
    </Line>
  );
}

export function PlusIcon({ size }: { size: number }) {
  return (
    <Line size={size} stroke="2">
      <path d="M12 5v14M5 12h14" />
    </Line>
  );
}

export function TagIcon() {
  return (
    <Line size={18} stroke="1.6">
      <path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z" />
      <circle cx="7.5" cy="7.5" r="1.3" />
    </Line>
  );
}

export function TagSmallIcon() {
  return (
    <Line size={12} stroke="2">
      <path d="M3 12V4a1 1 0 0 1 1-1h8l9 9-9 9z" />
      <circle cx="7.5" cy="7.5" r="1.3" />
    </Line>
  );
}

export function FunnelIcon() {
  return (
    <Line size={18} stroke="1.6">
      <path d="M4 5h16l-6 7.5V19l-4-2v-4.5z" />
    </Line>
  );
}

export function CalendarSmallIcon() {
  return (
    <Line size={12} stroke="2">
      <rect x="4" y="5" width="16" height="15" rx="2" />
      <path d="M4 10h16M8 3v4M16 3v4" />
    </Line>
  );
}

export function RepeatSmallIcon() {
  return (
    <Line size={12} stroke="2">
      <path d="M17 2l3 3-3 3" />
      <path d="M4 11V9a4 4 0 0 1 4-4h12" />
      <path d="M7 22l-3-3 3-3" />
      <path d="M20 13v2a4 4 0 0 1-4 4H4" />
    </Line>
  );
}

const FLAG_BANNER = "M5 4.5C7.5 3 10 3 12 4.5S16.5 6 19 4.5v9c-2.5 1.5-5 1.5-7 0s-4.5-1.5-7 0z";

export function FlagFilledIcon() {
  return (
    <Line size={12} stroke="2">
      <path d="M5 21V3" />
      <path d={FLAG_BANNER} fill="currentColor" />
    </Line>
  );
}

export function ChevronRightIcon() {
  return (
    <Line size={16} stroke="2">
      <path d="M9 6l6 6-6 6" />
    </Line>
  );
}

/** The completion circle's mark (the default "Waterline", `CompletionIcon::Wave`). */
export function DoneMark() {
  return (
    <span className="icon">
      <svg className="done-mark" viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M3.5 13 Q7 10 10.5 13 T17.5 13 T22 12"
          stroke="currentColor"
          strokeWidth="2.6"
          fill="none"
          strokeLinecap="round"
        />
      </svg>
    </span>
  );
}

const PINE =
  "M32 15 L36 20.6 Q35.2 21 34.3 20.9 L39 26.2 Q37.8 26.7 36.4 26.5 L41.4 31.2         Q32 29.6 22.6 31.2 L27.6 26.5 Q26.2 26.7 25 26.2 L29.7 20.9 Q28.8 21 28 20.6 Z";

function Tree() {
  return (
    <>
      <path
        d={PINE}
        transform="translate(32 15.3) scale(.82) translate(-32 -15)"
        fill="#fff"
        stroke="#fff"
        strokeWidth="1.6"
        strokeLinejoin="round"
      />
      <rect x="31" y="28" width="2" height="4" rx=".5" fill="#fff" />
    </>
  );
}

/** Keel's mark (icons.rs `logo`; keep in sync with crates/src-tauri/icons/keel.svg). */
export function Logo({ size }: { size: number }) {
  return (
    <span className="icon">
      <svg width={size} height={size} viewBox="0 0 64 64" fill="none" aria-hidden="true">
        <rect width="64" height="64" rx="16" fill="currentColor" />
        <circle cx="32" cy="32" r="22" stroke="#fff" strokeWidth="3" />
        <g>
          <Tree />
        </g>
        <g transform="translate(0 64) scale(1 -1)" opacity=".42">
          <Tree />
        </g>
        <path
          d="M10.6 32 Q16 30.6 21.5 32 T32.5 32 T43.5 32 T53.4 32"
          stroke="#fff"
          strokeWidth="3"
          strokeLinecap="round"
        />
      </svg>
    </span>
  );
}
