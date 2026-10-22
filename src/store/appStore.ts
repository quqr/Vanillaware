/** 应用级状态：平台标签、主题与游戏档案。
 *
 * 视图路由已并入库视图（ADR-0012）：不再有全局 `view` 与 `activeMbs`，
 * 目标文件由库的选中/打开状态决定，查看器直接接收 `path`。
 */

import { create } from "zustand";
import type { GameProfile } from "../lib/ipc";

/** 平台标签全集（与 `vw_formats::mbs::profile::lookup` 一致）。 */
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

/** 启用的 daisyUI 内置主题；与 `styles.css` 和 `index.html` 的白名单保持同步。 */
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

const THEME_KEY = "vw-theme";
const PROFILE_KEY = "vw-profile";

function readTheme(): ThemeId {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved && (THEMES as readonly string[]).includes(saved)) return saved as ThemeId;
  } catch {
    // localStorage 不可用时回退默认主题。
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
    // localStorage 不可用时回退自动嗅探。
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
  /** 用户手动改过 tag 后，档案嗅探不再覆盖。 */
  tagSource: "manual" | "profile";
  setProfile(profile: GameProfile | null): void;
  setProfileMode(mode: ProfileMode): void;
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
      // 持久化失败不阻塞切换。
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
      // 持久化失败不阻塞切换。
    }
    if (mode !== "auto") get().setProfile(PROFILE_PRESETS[mode]);
  },
}));
