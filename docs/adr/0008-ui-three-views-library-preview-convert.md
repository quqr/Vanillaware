# ADR-0008：UI 重构为「库 / 预览 / 转换」三视图，且不得与 QuadToSpine2D 雷同

- **状态**：部分废弃（2026-10-21，ADR-0012 取代「库沿用资产网格」与「全局三视图路由」；预览与转换合并为动画查看器的两个 tab）
- **影响范围**：`src/App.tsx` 及全部 `src/components`、`src/store`

## 背景

现状是 daisyUI 三栏资产浏览器：目录树 + 资产网格 + Inspector，功能上只覆盖「找文件」。
新需求要能看动画、能核格式，因此必须重构——不是加一个面板，而是重新划分视图。

`F:\Codes\QuadToSpine2D` 是同类工具（Avalonia 桌面，FileManager / Converter / Player 三页）。
用户明确要求**风格不要与它雷同**。

## 决策

- **三视图**：库（Library）/ 预览（Preview）/ 转换（Convert），顶栏或侧栏切换。
  - **库**：沿用现有网格 + 筛选 + Inspector 的能力，但按新设计语言重做。
  - **预览**：动画播放器，含时间轴、画布、图层面板（见 ADR-0007）。
  - **转换**：单文件 `V55 ↔ Quad` 字段级 diff（对齐 ADR-0006 的结构等价）与**导出产物**。
    不做批量回归跑，也不单独做 V55 浏览器——V55 结构树作为 diff 视图的一侧呈现。
- **差异化要求**：不得复用 QuadToSpine2D 的视觉语言（其深色三页桌面工具形态、标签与控件排列）。
  具体设计语言在实现前单独定稿，本 ADR 只锁定「三视图」这一结构与「必须差异化」这一约束。
- 重构是**重划分**而非增量叠加：旧的单页三栏布局不保留为兼容形态。

## 权衡

代价：一次性改动面大（App 骨架、store 分片、组件全量替换），短期回归风险高。

收益：预览与转换有了明确的归属空间，不至于把播放器塞进 Inspector 变成拼盘；
与同类工具拉开距离，避免「看起来是 QuadToSpine2D 的复刻」。

## 备选方案

- **双视图（库+检查器 / 全屏预览）**：改动小，但转换能力无处安放，最终会被塞回 Inspector。
- **单页工作台（左树 + 中画布 + 右时间轴）**：一屏完成，但在 722 个资产 + 多动画的规模下，
  导航与预览互相挤占空间。

---

## English summary

The UI is restructured into three views — Library / Preview / Convert. Library keeps the existing grid,
filtering and inspector capability under a new design language. Preview is the animation player with timeline,
canvas and layer panels. Convert handles single-file V55↔Quad field-level diff (structural equivalence, ADR-0006)
plus export; no batch regression run, no standalone V55 browser. The visual language must not reuse
QuadToSpine2D's dark three-page desktop look. This is a re-partition, not an additive panel — the old
single-page three-column layout is not kept for compatibility.
