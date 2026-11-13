// The app's icons: Phosphor (MIT) in bold, plain, on every platform. Filled
// weight only for the deadline and priority flags. Each renders inside a
// `span.icon`. Logo and DoneMark are Keel's own marks, and the tag, trophy, inbox
// and calendars are Keel's own rounder drawings of Phosphor's (see `Drawn`).
import { ArrowBendDownRight } from "@phosphor-icons/react/dist/csr/ArrowBendDownRight";
import { ArrowRight } from "@phosphor-icons/react/dist/csr/ArrowRight";
import { ArrowsDownUp } from "@phosphor-icons/react/dist/csr/ArrowsDownUp";
import { ArrowUp } from "@phosphor-icons/react/dist/csr/ArrowUp";
import { Bell } from "@phosphor-icons/react/dist/csr/Bell";
import { BellRinging } from "@phosphor-icons/react/dist/csr/BellRinging";
import { CaretDown } from "@phosphor-icons/react/dist/csr/CaretDown";
import { CaretLeft } from "@phosphor-icons/react/dist/csr/CaretLeft";
import { CaretRight } from "@phosphor-icons/react/dist/csr/CaretRight";
import { Check } from "@phosphor-icons/react/dist/csr/Check";
import { CheckCircle } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleDashed } from "@phosphor-icons/react/dist/csr/CircleDashed";
import { Clock } from "@phosphor-icons/react/dist/csr/Clock";
import { DotsThree } from "@phosphor-icons/react/dist/csr/DotsThree";
import { Fire } from "@phosphor-icons/react/dist/csr/Fire";
import { Flag } from "@phosphor-icons/react/dist/csr/Flag";
import { FlagCheckered } from "@phosphor-icons/react/dist/csr/FlagCheckered";
import { Funnel } from "@phosphor-icons/react/dist/csr/Funnel";
import { Gear } from "@phosphor-icons/react/dist/csr/Gear";
import { Hash } from "@phosphor-icons/react/dist/csr/Hash";
import { List } from "@phosphor-icons/react/dist/csr/List";
import { MagnifyingGlass } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { Minus } from "@phosphor-icons/react/dist/csr/Minus";
import { Monitor } from "@phosphor-icons/react/dist/csr/Monitor";
import { Moon } from "@phosphor-icons/react/dist/csr/Moon";
import { PencilSimple } from "@phosphor-icons/react/dist/csr/PencilSimple";
import { Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { Prohibit } from "@phosphor-icons/react/dist/csr/Prohibit";
import { Repeat } from "@phosphor-icons/react/dist/csr/Repeat";
import { Rows } from "@phosphor-icons/react/dist/csr/Rows";
import { Sun } from "@phosphor-icons/react/dist/csr/Sun";
import { Target } from "@phosphor-icons/react/dist/csr/Target";
import { Trash } from "@phosphor-icons/react/dist/csr/Trash";
import { X } from "@phosphor-icons/react/dist/csr/X";
import type { Icon } from "@phosphor-icons/react/dist/lib/types";
import type { ReactNode } from "react";

/** Extra line weight for `heavier` icons, on Phosphor's 256 grid (bold lines are 24). */
const HEAVIER_STROKE = 6;

function Ph({
  icon: Glyph,
  size,
  weight = "bold",
  heavier = false,
  children,
}: {
  icon: Icon;
  size: number;
  weight?: "bold" | "fill";
  heavier?: boolean;
  children?: ReactNode;
}) {
  const stroke = heavier
    ? { stroke: "currentColor", strokeWidth: HEAVIER_STROKE, strokeLinejoin: "round" as const }
    : {};
  return (
    <span className="icon">
      <Glyph size={size} weight={weight} aria-hidden="true" {...stroke}>
        {children}
      </Glyph>
    </span>
  );
}

/** Line weight and ends for `Drawn` icons, matching Phosphor's bold. */
const LINE = {
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 24,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

/** An icon Keel draws itself, on Phosphor's 256 grid. */
function Drawn({ size, children }: { size: number; children: ReactNode }) {
  return (
    <span className="icon">
      <svg width={size} height={size} viewBox="0 0 256 256" fill="currentColor" aria-hidden="true">
        {children}
      </svg>
    </span>
  );
}

function CalendarFrame() {
  return (
    <g {...LINE}>
      <rect x="40" y="40" width="176" height="176" rx="20" />
      <path d="M40 88H216M80 24V56M176 24V56" />
    </g>
  );
}

function TagShape() {
  return (
    <>
      <path
        {...LINE}
        d="M40 68Q40 40 68 40L124.4 40Q136 40 144.2 48.2L223.2 127.2Q243 147 223.2 166.8L166.8 223.2Q147 243 127.2 223.2L48.2 144.2Q40 136 40 124.4Z"
      />
      <circle cx="88" cy="88" r="16" />
    </>
  );
}

export function FireIcon({ size = 16 }: { size?: number }) {
  return <Ph icon={Fire} size={size} />;
}

/** The deadline's flag. */
export function FlagFillIcon({ size = 14 }: { size?: number }) {
  return <Ph icon={Flag} size={size} weight="fill" />;
}

export function PlusIcon({ size }: { size: number }) {
  return <Ph icon={Plus} size={size} heavier />;
}

/** Calendar showing today's day-of-month. */
export function TodayIcon({ day }: { day: number }) {
  return (
    <Drawn size={20}>
      <CalendarFrame />
      <text x="128" y="190" textAnchor="middle" fontSize="96" fontWeight="700" fontFamily="sans-serif">
        {day}
      </text>
    </Drawn>
  );
}

export function InboxIcon() {
  return (
    <Drawn size={20}>
      <g {...LINE}>
        <rect x="40" y="40" width="176" height="176" rx="20" />
        <path d="M40 156H76.7a8 8 0 0 1 5.66 2.34L100.7 176.7a8 8 0 0 0 5.66 2.34h43.3a8 8 0 0 0 5.66-2.34l18.3-18.3A8 8 0 0 1 179.3 156H216" />
      </g>
    </Drawn>
  );
}

export function UpcomingIcon() {
  return (
    <Drawn size={20}>
      <CalendarFrame />
      <circle cx="128" cy="128" r="16" />
      <circle cx="176" cy="128" r="16" />
      <circle cx="80" cy="176" r="16" />
      <circle cx="128" cy="176" r="16" />
      <circle cx="176" cy="176" r="16" />
    </Drawn>
  );
}

export function SearchIcon() {
  return <Ph icon={MagnifyingGlass} size={20} heavier />;
}

export function MenuIcon() {
  return <Ph icon={List} size={20} heavier />;
}

export function MonitorIcon() {
  return <Ph icon={Monitor} size={18} />;
}

export function GearIcon() {
  return <Ph icon={Gear} size={18} />;
}

export function TagIcon() {
  return (
    <Drawn size={18}>
      <TagShape />
    </Drawn>
  );
}

export function TagSmallIcon() {
  return (
    <Drawn size={12}>
      <TagShape />
    </Drawn>
  );
}

export function TrophySmallIcon() {
  return (
    <Drawn size={12}>
      <path
        {...LINE}
        d="M76 44H180Q196 44 196 60V112A68 68 0 0 1 60 112V60Q60 44 76 44ZM60 68H36Q20 68 20 84V92Q20 132 64 136M196 68H220Q236 68 236 84V92Q236 132 192 136M128 180V216M88 216H168"
      />
    </Drawn>
  );
}

export function FunnelIcon() {
  return <Ph icon={Funnel} size={18} />;
}

export function SortIcon() {
  return <Ph icon={ArrowsDownUp} size={18} />;
}

export function CalendarSmallIcon() {
  return (
    <Drawn size={12}>
      <CalendarFrame />
    </Drawn>
  );
}

export function MoreIcon() {
  return <Ph icon={DotsThree} size={18} heavier />;
}

export function RepeatSmallIcon() {
  return <Ph icon={Repeat} size={12} />;
}

export function FlagFilledIcon() {
  return <Ph icon={Flag} size={12} weight="fill" />;
}

export function ClockSmallIcon() {
  return <Ph icon={Clock} size={12} />;
}

export function HashSmallIcon() {
  return <Ph icon={Hash} size={14} />;
}

/** A section. */
export function SectionSmallIcon() {
  return <Ph icon={Rows} size={14} />;
}

export function ChevronLeftIcon() {
  return <Ph icon={CaretLeft} size={16} />;
}

export function ChevronDownSmallIcon() {
  return <Ph icon={CaretDown} size={12} />;
}

export function ArrowRightIcon() {
  return <Ph icon={ArrowRight} size={20} />;
}

export function ArrowUpIcon() {
  return <Ph icon={ArrowUp} size={22} />;
}

export function NoDateIcon() {
  return <Ph icon={Prohibit} size={16} />;
}

export function ChevronRightIcon() {
  return <Ph icon={CaretRight} size={16} />;
}

export function CheckIcon() {
  return <Ph icon={Check} size={11} />;
}

/** The dash of a check that's partly ticked (a group with some of its names). */
export function MinusIcon() {
  return <Ph icon={Minus} size={11} />;
}

export function CheckSmallIcon() {
  return <Ph icon={Check} size={14} />;
}

export function CheckCircleIcon() {
  return <Ph icon={CheckCircle} size={16} heavier />;
}

export function TargetSmallIcon() {
  return <Ph icon={Target} size={12} />;
}

export function FlagCheckeredSmallIcon() {
  return <Ph icon={FlagCheckered} size={12} />;
}

export function BellSmallIcon() {
  return <Ph icon={Bell} size={12} />;
}

export function SubtaskIcon() {
  return <Ph icon={ArrowBendDownRight} size={18} />;
}

export function TrashIcon() {
  return <Ph icon={Trash} size={18} />;
}

export function TrashSmallIcon() {
  return <Ph icon={Trash} size={16} />;
}

export function PencilIcon() {
  return <Ph icon={PencilSimple} size={16} />;
}

export function SunIcon() {
  return <Ph icon={Sun} size={18} heavier />;
}

export function MoonIcon() {
  return <Ph icon={Moon} size={18} heavier />;
}

export function XIcon() {
  return <Ph icon={X} size={16} />;
}

export function XSmallIcon() {
  return <Ph icon={X} size={12} />;
}

export function CircleDashedIcon() {
  return <Ph icon={CircleDashed} size={15} />;
}

export function BellRingingIcon() {
  return <Ph icon={BellRinging} size={14} />;
}

/** The mark a completing circle fills with (`CompletionIcon::svg`, crates/ui/src/completion.rs). */
export function DoneMark({ icon = "wave" }: { icon?: "wave" | "pine" | "reflection" }) {
  if (icon === "pine") {
    return (
      <span className="icon">
        <svg className="done-mark" viewBox="20 12 24 22" aria-hidden="true">
          <path d={PINE} fill="currentColor" stroke="currentColor" strokeWidth="1.4" strokeLinejoin="round" />
          <rect x="31" y="30" width="2" height="3" rx=".5" fill="currentColor" />
        </svg>
      </span>
    );
  }
  if (icon === "reflection") {
    const pine = "translate(32 15.3) scale(.82) translate(-32 -15)";
    return (
      <span className="icon">
        <svg className="done-mark" viewBox="16 10 32 44" aria-hidden="true">
          <g transform={pine}>
            <path
              d={PINE}
              fill="currentColor"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinejoin="round"
            />
          </g>
          <g transform="translate(0 64) scale(1 -1)" opacity=".45">
            <g transform={pine}>
              <path d={PINE} fill="currentColor" />
            </g>
          </g>
          <path
            d="M17 32 Q22 30.6 27 32 T37 32 T47 32"
            stroke="currentColor"
            strokeWidth="3"
            fill="none"
            strokeLinecap="round"
          />
        </svg>
      </span>
    );
  }
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
