import type { ReactNode } from "react";

import type { Tint } from "@/components/IconBadge";
import {
  AiIcon,
  CaptionIcon,
  FolderIcon,
  HandIcon,
  HomeIcon,
  InfoIcon,
  KeyIcon,
  KeyboardIcon,
  LibraryIcon,
  MicIcon,
  SlidersIcon,
  WaveformIcon,
} from "@/components/icons";

/**
 * The application's information architecture.
 *
 * Two places to use Audis, then its settings. Home is where everything starts
 * and where a running session lives; Library is what sessions leave behind.
 * Settings are grouped the way System Settings groups panes, one pane per thing
 * a person would go looking for.
 */

export type ViewId =
  | "home"
  | "library"
  | "general"
  | "audio"
  | "speech"
  | "captions"
  | "assistant"
  | "providers"
  | "shortcuts"
  | "storage"
  | "privacy"
  | "about";

export interface NavItem {
  id: ViewId;
  label: string;
  icon: ReactNode;
  tint: Tint;
  /** Words someone might search for to find this pane, beyond its name. */
  keywords: readonly string[];
}

export interface NavGroup {
  /** Group heading. Omitted for groups that need no label. */
  title?: string;
  items: readonly NavItem[];
}

export const NAV_GROUPS: readonly NavGroup[] = [
  {
    items: [
      {
        id: "home",
        label: "Home",
        icon: <HomeIcon />,
        tint: "blue",
        keywords: ["start", "quick start", "dashboard", "features", "session", "live caption"],
      },
      {
        id: "library",
        label: "Library",
        icon: <LibraryIcon />,
        tint: "orange",
        keywords: ["sessions", "transcripts", "history", "export", "report", "srt", "markdown"],
      },
    ],
  },
  {
    title: "Settings",
    items: [
      {
        id: "general",
        label: "General",
        icon: <SlidersIcon />,
        tint: "gray",
        keywords: ["theme", "appearance", "dark", "light", "startup", "open on", "tray", "close"],
      },
      {
        id: "audio",
        label: "Audio",
        icon: <MicIcon />,
        tint: "red",
        keywords: [
          "microphone",
          "device",
          "computer audio",
          "loopback",
          "level",
          "test",
          "sources",
        ],
      },
      {
        id: "speech",
        label: "Speech Recognition",
        icon: <WaveformIcon />,
        tint: "purple",
        keywords: [
          "transcription",
          "engine",
          "whisper",
          "models",
          "download",
          "language",
          "indonesian",
          "english",
          "speakers",
          "separation",
          "cloud",
        ],
      },
      {
        id: "captions",
        label: "Captions",
        icon: <CaptionIcon />,
        tint: "teal",
        keywords: ["overlay", "font", "text size", "lines", "background", "click through"],
      },
      {
        id: "assistant",
        label: "AI Assistant",
        icon: <AiIcon />,
        tint: "indigo",
        keywords: ["ai", "questions", "answers", "context", "notes", "meeting", "interview", "ask"],
      },
      {
        id: "providers",
        label: "AI Providers",
        icon: <KeyIcon />,
        tint: "green",
        keywords: [
          "api key",
          "key",
          "gemini",
          "groq",
          "deepgram",
          "deepseek",
          "anthropic",
          "claude",
          "openai",
          "endpoint",
        ],
      },
      {
        id: "shortcuts",
        label: "Shortcuts",
        icon: <KeyboardIcon />,
        tint: "pink",
        keywords: ["hotkey", "keyboard", "keys"],
      },
      {
        id: "storage",
        label: "Storage",
        icon: <FolderIcon />,
        tint: "gray",
        keywords: ["files", "disk", "space", "recordings", "logs", "exports", "folder"],
      },
      {
        id: "privacy",
        label: "Privacy",
        icon: <HandIcon />,
        tint: "blue",
        keywords: ["consent", "data", "analytics", "local"],
      },
    ],
  },
  {
    items: [
      {
        id: "about",
        label: "About Audis",
        icon: <InfoIcon />,
        tint: "gray",
        keywords: ["version", "update", "diagnostics", "logs", "webview", "release", "beta"],
      },
    ],
  },
];

const ALL_ITEMS = NAV_GROUPS.flatMap((group) => group.items);

/** Look up a nav item by id. */
export function findNavItem(id: ViewId): NavItem | undefined {
  return ALL_ITEMS.find((item) => item.id === id);
}

/** Every item whose name or keywords match `query`, best match first. */
export function searchNav(query: string): NavItem[] {
  const needle = query.trim().toLowerCase();
  if (!needle) return ALL_ITEMS;

  const scored = ALL_ITEMS.map((item) => {
    const label = item.label.toLowerCase();
    const score = label.startsWith(needle)
      ? 3
      : label.includes(needle)
        ? 2
        : item.keywords.some((keyword) => keyword.includes(needle))
          ? 1
          : 0;
    return { item, score };
  });

  return scored
    .filter((entry) => entry.score > 0)
    .sort((a, b) => b.score - a.score)
    .map((entry) => entry.item);
}

/** Navigate to a page, and optionally to a group on it by its anchor id. */
export type Navigate = (id: ViewId, anchor?: string) => void;
