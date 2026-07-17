import type { ReactNode } from "react";

import {
  AiIcon,
  SparkIcon,
  BellIcon,
  CaptionIcon,
  ClockIcon,
  FolderIcon,
  GaugeIcon,
  InfoIcon,
  KeyIcon,
  KeyboardIcon,
  LockIcon,
  MicIcon,
  SlidersIcon,
  SpeakerIcon,
  StethoscopeIcon,
  TextIcon,
} from "@/components/icons";

/** The application's information architecture. */

export type ViewId =
  | "dashboard"
  | "features"
  | "sessions"
  | "files"
  | "general"
  | "audio"
  | "transcription"
  | "speakers"
  | "assistant"
  | "captions"
  | "shortcuts"
  | "providers"
  | "models"
  | "storage"
  | "privacy"
  | "diagnostics"
  | "about";

export interface NavItem {
  id: ViewId;
  label: string;
  icon: ReactNode;
  /** Page title shown in the header. */
  title: string;
}

export interface NavGroup {
  /** Group heading. Omitted for the first group, which needs no label. */
  title?: string;
  items: readonly NavItem[];
}

export const NAV_GROUPS: readonly NavGroup[] = [
  {
    items: [
      { id: "dashboard", label: "Dashboard", icon: <GaugeIcon />, title: "Dashboard" },
      { id: "features", label: "Features", icon: <SparkIcon />, title: "Features" },
      { id: "sessions", label: "Sessions", icon: <ClockIcon />, title: "Sessions" },
      { id: "files", label: "Files", icon: <FolderIcon />, title: "Files" },
    ],
  },
  {
    title: "Settings",
    items: [
      { id: "general", label: "General", icon: <SlidersIcon />, title: "General" },
      { id: "audio", label: "Audio", icon: <MicIcon />, title: "Audio" },
      {
        id: "transcription",
        label: "Transcription",
        icon: <TextIcon />,
        title: "Transcription",
      },
      { id: "speakers", label: "Speakers", icon: <SpeakerIcon />, title: "Speakers" },
      { id: "assistant", label: "AI Assistant", icon: <AiIcon />, title: "AI Assistant" },
      { id: "captions", label: "Captions", icon: <CaptionIcon />, title: "Captions" },
      { id: "shortcuts", label: "Shortcuts", icon: <KeyboardIcon />, title: "Shortcuts" },
      { id: "providers", label: "Providers", icon: <KeyIcon />, title: "Providers" },
      { id: "models", label: "Models", icon: <BellIcon />, title: "Local models" },
      { id: "storage", label: "Storage", icon: <FolderIcon />, title: "Storage" },
      { id: "privacy", label: "Privacy", icon: <LockIcon />, title: "Privacy" },
      {
        id: "diagnostics",
        label: "Diagnostics",
        icon: <StethoscopeIcon />,
        title: "Diagnostics",
      },
    ],
  },
  {
    items: [{ id: "about", label: "About", icon: <InfoIcon />, title: "About Audis" }],
  },
];

const ALL_ITEMS = NAV_GROUPS.flatMap((group) => group.items);

/** Look up a nav item by id. */
export function findNavItem(id: ViewId): NavItem | undefined {
  return ALL_ITEMS.find((item) => item.id === id);
}
