import { create } from "zustand";
import { setLocale as applyLocale, type Locale } from "../i18n";
import type { GameProfile } from "../lib/ipc";

export const TAGS = [
  "swi_unic",
  "ps4_unic",
  "ps5_unic",
  "xbx_unic",
  "swi_sent",
  "ps4_sent",
  "swi_grim",
  "ps4_grim",
  "ps5_grim",
  "ps3_odin",
  "ps4_odin",
  "ps4_drag",
] as const;

export type TagId = (typeof TAGS)[number];

export const THEMES = [
  "dark",
  "light",
  "corporate",
  "business",
  "nord",
  "dim",
  "night",
  "dracula",
] as const;

export type ThemeId = (typeof THEMES)[number];

const THEME_KEY = "theme";
const PROFILE_KEY = "profile";
const LOCALE_KEY = "locale";

function readLocale(): Locale {
  try {
    const saved = localStorage.getItem(LOCALE_KEY);
    if (saved === "en" || saved === "zh") {
      applyLocale(saved);
      return saved;
    }
  } catch {
  }
  return "en";
}

function readTheme(): ThemeId {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved && (THEMES as readonly string[]).includes(saved)) return saved as ThemeId;
  } catch {
  }
  return "dark";
}

export type ProfileMode = "auto" | "unicorn_overlord" | "thirteen_sentinels";

export const PROFILE_PRESETS: Record<Exclude<ProfileMode, "auto">, GameProfile> = {
  unicorn_overlord: {
    id: "unicorn_overlord",
    label: "Unicorn Overlord",
    mbsTag: "swi_unic",
    confidence: 100,
    evidence: [],
  },
  thirteen_sentinels: {
    id: "thirteen_sentinels",
    label: "13 Sentinels",
    mbsTag: "swi_sent",
    confidence: 100,
    evidence: [],
  },
};

function readProfileMode(): ProfileMode {
  try {
    const saved = localStorage.getItem(PROFILE_KEY);
    if (saved === "auto" || saved === "unicorn_overlord" || saved === "thirteen_sentinels") {
      return saved;
    }
  } catch {
  }
  return "auto";
}

interface AppState {
  tag: TagId;
  setTag(tag: TagId): void;

  theme: ThemeId;
  setTheme(theme: ThemeId): void;

  profile: GameProfile | null;
  profileMode: ProfileMode;

  tagSource: "manual" | "profile";
  setProfile(profile: GameProfile | null): void;
  setProfileMode(mode: ProfileMode): void;

  locale: Locale;
  setLocale(locale: Locale): void;
}

export const useAppStore = create<AppState>((set, get) => ({
  tag: "swi_unic",

  setTag(tag) {
    set({ tag, tagSource: "manual" });
  },

  theme: readTheme(),

  setTheme(theme) {
    set({ theme });
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem(THEME_KEY, theme);
    } catch {
    }
  },

  locale: readLocale(),

  setLocale(locale) {
    set({ locale });
    applyLocale(locale);
    try {
      localStorage.setItem(LOCALE_KEY, locale);
    } catch {
    }
  },

  profile: null,
  profileMode: readProfileMode(),
  tagSource: "profile",

  setProfile(profile) {
    set((prev) => {
      const patch: Partial<AppState> = { profile };
      const mbsTag = profile?.mbsTag;
      if (
        prev.tagSource !== "manual" &&
        mbsTag &&
        (TAGS as readonly string[]).includes(mbsTag)
      ) {
        patch.tag = mbsTag as TagId;
        patch.tagSource = "profile";
      }
      return patch;
    });
  },

  setProfileMode(mode) {
    set({ profileMode: mode });
    try {
      localStorage.setItem(PROFILE_KEY, mode);
    } catch {
    }
    if (mode !== "auto") get().setProfile(PROFILE_PRESETS[mode]);
  },
}));
