/** 应用级状态：平台标签。
 *
 * 视图路由已并入库视图（ADR-0012）：不再有全局 `view` 与 `activeMbs`，
 * 目标文件由库的选中/打开状态决定，查看器直接接收 `path`。
 */

import { create } from "zustand";

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
}

export const useAppStore = create<AppState>((set) => ({
  tag: "swi_unic",

  setTag(tag) {
    set({ tag });
  },
}));
