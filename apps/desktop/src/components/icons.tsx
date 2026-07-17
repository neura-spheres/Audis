/** Line icons matching SF Symbols' proportions: 1.5px strokes on a 16px grid, */

const base = {
  width: 16,
  height: 16,
  viewBox: "0 0 16 16",
  fill: "none",
  stroke: "currentColor",
  strokeWidth: 1.5,
  strokeLinecap: "round",
  strokeLinejoin: "round",
} as const;

export function GaugeIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M2.5 11.5a6 6 0 1 1 11 0" />
      <path d="M8 8.5 11 6" />
    </svg>
  );
}

export function InfoIcon() {
  return (
    <svg {...base} aria-hidden>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 7.25v4M8 4.9v.1" />
    </svg>
  );
}

export function FolderIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h2.2a1 1 0 0 1 .8.4l.7.95a1 1 0 0 0 .8.4h4.5A1.5 1.5 0 0 1 14 6.25v5.25A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5z" />
    </svg>
  );
}

export function ClockIcon() {
  return (
    <svg {...base} aria-hidden>
      <circle cx="8" cy="8" r="6" />
      <path d="M8 4.75V8l2.25 1.5" />
    </svg>
  );
}

export function SlidersIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M2.5 5h7M12 5h1.5M2.5 11h1.5M6.5 11h7" />
      <circle cx="10.75" cy="5" r="1.5" />
      <circle cx="5.25" cy="11" r="1.5" />
    </svg>
  );
}

export function MicIcon() {
  return (
    <svg {...base} aria-hidden>
      <rect x="6" y="2" width="4" height="7" rx="2" />
      <path d="M3.75 7.5a4.25 4.25 0 0 0 8.5 0M8 11.75V14" />
    </svg>
  );
}

export function TextIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M3 4h10M3 8h10M3 12h6" />
    </svg>
  );
}

export function SpeakerIcon() {
  return (
    <svg {...base} aria-hidden>
      <circle cx="6" cy="5.5" r="2.5" />
      <path d="M2 13a4 4 0 0 1 8 0" />
      <path d="M11.5 4.5a4 4 0 0 1 0 6" />
    </svg>
  );
}

export function AiIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M8 2l1.4 3.6L13 7l-3.6 1.4L8 12l-1.4-3.6L3 7l3.6-1.4z" />
      <path d="M12.5 11.5l.5 1.25L14.25 13.25l-1.25.5L12.5 15l-.5-1.25L10.75 13.25l1.25-.5z" />
    </svg>
  );
}

export function CaptionIcon() {
  return (
    <svg {...base} aria-hidden>
      <rect x="2" y="3.5" width="12" height="9" rx="2" />
      <path d="M5 8.5h2.5M9 8.5h2" />
    </svg>
  );
}

export function KeyboardIcon() {
  return (
    <svg {...base} aria-hidden>
      <rect x="1.75" y="4" width="12.5" height="8" rx="1.5" />
      <path d="M4.5 6.75v.01M7 6.75v.01M9.5 6.75v.01M5 9.5h6" />
    </svg>
  );
}

export function KeyIcon() {
  return (
    <svg {...base} aria-hidden>
      <circle cx="5" cy="8" r="2.75" />
      <path d="M7.75 8H14M11.5 8v2.25M13.5 8v1.75" />
    </svg>
  );
}

export function BellIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M4 7a4 4 0 0 1 8 0c0 3 1 4 1 4H3s1-1 1-4z" />
      <path d="M6.75 13.25a1.5 1.5 0 0 0 2.5 0" />
    </svg>
  );
}

export function LockIcon() {
  return (
    <svg {...base} aria-hidden>
      <rect x="3.25" y="7" width="9.5" height="6.5" rx="1.5" />
      <path d="M5.75 7V5.25a2.25 2.25 0 0 1 4.5 0V7" />
    </svg>
  );
}

export function StethoscopeIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M4.5 2.5v3.75a2.75 2.75 0 0 0 5.5 0V2.5" />
      <path d="M7.25 9v1.5a3 3 0 0 0 6 0v-1" />
      <circle cx="13.25" cy="8" r="1.25" />
    </svg>
  );
}

export function ExternalIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M9.5 3h3.5v3.5" />
      <path d="M13 3 8 8" />
      <path d="M12 9.75v2.75a1.5 1.5 0 0 1-1.5 1.5h-7A1.5 1.5 0 0 1 2 12.5v-7A1.5 1.5 0 0 1 3.5 4h2.75" />
    </svg>
  );
}

export function RevealIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h2.2a1 1 0 0 1 .8.4l.7.95a1 1 0 0 0 .8.4h4.5A1.5 1.5 0 0 1 14 6.25v5.25A1.5 1.5 0 0 1 12.5 13h-9A1.5 1.5 0 0 1 2 11.5z" />
      <path d="M8 7.5v3M6.5 9 8 10.5 9.5 9" />
    </svg>
  );
}

export function SparkIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M8 2.25l1.5 3.9 3.9 1.5-3.9 1.5L8 13.05 6.5 9.15 2.6 7.65l3.9-1.5z" />
    </svg>
  );
}

export function CheckIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M3 8.5 6.25 11.75 13 5" />
    </svg>
  );
}

export function DownloadIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M8 2.75v7.5M5 7.5 8 10.5l3-3" />
      <path d="M2.75 11.5v1a1.5 1.5 0 0 0 1.5 1.5h7.5a1.5 1.5 0 0 0 1.5-1.5v-1" />
    </svg>
  );
}

export function TrashIcon() {
  return (
    <svg {...base} aria-hidden>
      <path d="M2.75 4.5h10.5M6.5 4.5V3.25a1 1 0 0 1 1-1h1a1 1 0 0 1 1 1V4.5" />
      <path d="M4.25 4.5l.6 8a1 1 0 0 0 1 .95h4.3a1 1 0 0 0 1-.95l.6-8" />
    </svg>
  );
}

export function ConstructionIcon() {
  return (
    <svg {...base} width={20} height={20} viewBox="0 0 16 16" aria-hidden>
      <path d="M2 12.5h12" />
      <path d="M3.5 12.5V8.25a4.5 4.5 0 0 1 9 0v4.25" />
      <path d="M8 3.75V2M12.5 5.5l1.25-1.25M3.5 5.5 2.25 4.25" />
    </svg>
  );
}
