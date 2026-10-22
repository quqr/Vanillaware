/** 应用级状态：平台标签与主题。
 *
 * 视图路由已并入库视图（ADR-0012）：不再有全局 `view` 与 `activeMbs`，
 * 目标文件由库的选中/打开状态决定，查看器直接接收 `path`。
 */

import { create } from "zustand";

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

function readTheme(): ThemeId {
  try {
    const saved = localStorage.getItem(THEME_KEY);
    if (saved && (THEMES as readonly string[]).includes(saved)) return saved as ThemeId;
  } catch {
    // localStorage 不可用时回退默认主题。
  }
  return "dark";
}

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

interface AppState {
  tag: TagId;
  setTag(tag: TagId): void;
  theme: ThemeId;
  setTheme(theme: ThemeId): void;
}

export const useAppStore = create<AppState>((set) => ({
  tag: "swi_unic",

  setTag(tag) {
    set({ tag });
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
}));
