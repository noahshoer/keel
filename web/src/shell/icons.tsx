// The app's icons: Phosphor (MIT) in bold, plain, on every platform. Filled
// weight only for the deadline and priority flags. Each renders inside a
// `span.icon`. Logo and DoneMark are Keel's own marks.
import { ArrowBendDownRight } from "@phosphor-icons/react/dist/csr/ArrowBendDownRight";
import { ArrowRight } from "@phosphor-icons/react/dist/csr/ArrowRight";
import { Bell } from "@phosphor-icons/react/dist/csr/Bell";
import { BellRinging } from "@phosphor-icons/react/dist/csr/BellRinging";
import { CalendarBlank } from "@phosphor-icons/react/dist/csr/CalendarBlank";
import { CalendarDots } from "@phosphor-icons/react/dist/csr/CalendarDots";
import { CaretDown } from "@phosphor-icons/react/dist/csr/CaretDown";
import { CaretLeft } from "@phosphor-icons/react/dist/csr/CaretLeft";
import { CaretRight } from "@phosphor-icons/react/dist/csr/CaretRight";
import { Check } from "@phosphor-icons/react/dist/csr/Check";
import { CheckCircle } from "@phosphor-icons/react/dist/csr/CheckCircle";
import { CircleDashed } from "@phosphor-icons/react/dist/csr/CircleDashed";
import { Clock } from "@phosphor-icons/react/dist/csr/Clock";
import { Couch } from "@phosphor-icons/react/dist/csr/Couch";
import { DotsThree } from "@phosphor-icons/react/dist/csr/DotsThree";
import { Fire } from "@phosphor-icons/react/dist/csr/Fire";
import { Flag } from "@phosphor-icons/react/dist/csr/Flag";
import { Funnel } from "@phosphor-icons/react/dist/csr/Funnel";
import { Gear } from "@phosphor-icons/react/dist/csr/Gear";
import { Hash } from "@phosphor-icons/react/dist/csr/Hash";
import { List } from "@phosphor-icons/react/dist/csr/List";
import { MagnifyingGlass } from "@phosphor-icons/react/dist/csr/MagnifyingGlass";
import { Monitor } from "@phosphor-icons/react/dist/csr/Monitor";
import { Moon } from "@phosphor-icons/react/dist/csr/Moon";
import { PencilSimple } from "@phosphor-icons/react/dist/csr/PencilSimple";
import { Plus } from "@phosphor-icons/react/dist/csr/Plus";
import { Prohibit } from "@phosphor-icons/react/dist/csr/Prohibit";
import { Repeat } from "@phosphor-icons/react/dist/csr/Repeat";
import { Rows } from "@phosphor-icons/react/dist/csr/Rows";
import { Sun } from "@phosphor-icons/react/dist/csr/Sun";
import { Tag } from "@phosphor-icons/react/dist/csr/Tag";
import { Target } from "@phosphor-icons/react/dist/csr/Target";
import { TextAlignLeft } from "@phosphor-icons/react/dist/csr/TextAlignLeft";
import { Trash } from "@phosphor-icons/react/dist/csr/Trash";
import { Tray } from "@phosphor-icons/react/dist/csr/Tray";
import { X } from "@phosphor-icons/react/dist/csr/X";
import type { Icon } from "@phosphor-icons/react/dist/lib/types";
import type { ReactNode } from "react";

function Ph({
  icon: Glyph,
  size,
  weight = "bold",
  children,
}: {
  icon: Icon;
  size: number;
  weight?: "bold" | "fill";
  children?: ReactNode;
}) {
  return (
    <span className="icon">
      <Glyph size={size} weight={weight} aria-hidden="true">
        {children}
      </Glyph>
    </span>
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
  return <Ph icon={Plus} size={size} />;
}

/** Calendar showing today's day-of-month. */
export function TodayIcon({ day }: { day: number }) {
  return (
    <Ph icon={CalendarBlank} size={20}>
      <text x="128" y="190" textAnchor="middle" fontSize="96" fontWeight="700" fontFamily="sans-serif">
        {day}
      </text>
    </Ph>
  );
}

export function InboxIcon() {
  return <Ph icon={Tray} size={20} />;
}

export function UpcomingIcon() {
  return <Ph icon={CalendarDots} size={20} />;
}

export function SearchIcon() {
  return <Ph icon={MagnifyingGlass} size={20} />;
}

export function MenuIcon() {
  return <Ph icon={List} size={20} />;
}

export function MonitorIcon() {
  return <Ph icon={Monitor} size={18} />;
}

export function GearIcon() {
  return <Ph icon={Gear} size={18} />;
}

export function TagIcon() {
  return <Ph icon={Tag} size={18} />;
}

export function TagSmallIcon() {
  return <Ph icon={Tag} size={12} />;
}

export function FunnelIcon() {
  return <Ph icon={Funnel} size={18} />;
}

export function CalendarSmallIcon() {
  return <Ph icon={CalendarBlank} size={12} />;
}

/** The task view's date row. Bold like the rest; only flags are filled. */
export function CalendarFilledIcon() {
  return <Ph icon={CalendarBlank} size={12} />;
}

export function MoreIcon() {
  return <Ph icon={DotsThree} size={18} />;
}

/** A paragraph: the description's row. */
export function TextLinesIcon() {
  return <Ph icon={TextAlignLeft} size={16} />;
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

export function FlagSmallIcon() {
  return <Ph icon={Flag} size={12} />;
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

export function SofaIcon() {
  return <Ph icon={Couch} size={16} />;
}

export function ArrowRightIcon() {
  return <Ph icon={ArrowRight} size={16} />;
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

export function CheckSmallIcon() {
  return <Ph icon={Check} size={14} />;
}

export function CheckCircleIcon() {
  return <Ph icon={CheckCircle} size={16} />;
}

export function TargetSmallIcon() {
  return <Ph icon={Target} size={12} />;
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
  return <Ph icon={Sun} size={18} />;
}

export function MoonIcon() {
  return <Ph icon={Moon} size={18} />;
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
