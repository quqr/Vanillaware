# ADR-0012：库重构为「目录树 + 文件条目 + 查看器」，配对降为派生关联

- **状态**：已接受（2026-10-21）
- **影响范围**：`crates/vw-core`（扫描层）、`src-tauri`（命令层与状态）、`src/views`、`src/components`、`src/store`
- **取代**：ADR-0001（资产单元配对）全部；ADR-0008（三视图）中「库视图沿用资产网格」与「全局三视图路由」部分

## 背景

库当前的主对象是**资产单元**：`crates/vw-core/src/scan.rs` 的 `side_of()` 只认 `.ftx/.ftp` 与 `.mbs/.mbp`，
其他文件在扫描阶段直接丢弃；命中项再按 `(目录, 基础名)` 归并成 `AssetUnit`，前端「一个单元一张卡」。

真实语料（`UNICORN OVERLORD\extract`，10,234 文件 / 63 目录 / 41GB）里还有 `.nnd`(302)、`.bbs`(152)、
`.asb`(680)、`.fms`(32)、`.mpd`(25)、`.acb` 等格式，它们**完全不可见**；只含未识别文件的目录、
空目录也不出现在目录树里（目录树是从单元反推的）。

后续任务会持续增加二进制解析器，这意味着「哪些文件可见」不能由解析器的实现进度决定。

## 决策

- **主对象改为文件条目**：一条 = 磁盘上一个真实文件，以绝对路径为键。**收录范围只限原生游戏二进制资源**：
  注册表登记过的扩展名才出现，`.png` / `.json` / `.txt` 这类派生产物与外部产物不进库（只报数量）。
- **目录树改为真实目录**：懒加载子节点，空目录照常显示，与文件列表同源（`read_dir`）。
  提供面包屑、后退/前进/上一级，以及「仅含可识别格式」的视觉折叠开关（默认关，不删节点）。
- **配对降为派生关联**：同目录下同基础名的文件互为「关联文件」，在详情里列出，
  既不决定列表分组，也不决定可见性。
- **查看器按类别路由**：库占左侧（目录 + 列表/网格），右侧是查看器面板，按文件的类别渲染。
  同一文件可挂多个查看器（动画 = 「播放」+「结构」两个 tab），平台 `tag` 选择器移到查看器头部。
- **旧的扫描契约删除**：`scan_folder` / `AssetUnit` / `enrich_counts` 及其 Rust 类型与测试一并移除，
  不保留双轨。新增 `list_dir` / `file_meta` / `file_thumb` / `search_files` / `index_status`。

## 权衡

代价：数据模型、后端扫描层、store 与几乎全部库组件要重写；短期回归面大。
网格视图与列表视图两套渲染 + 选中态同步，是本次额外的实现成本。

收益：可见性不再被解析器进度绑架；新增格式只需注册表一条 + 一个查看器组件；
目录导航符合用户对「文件夹浏览器」的既有心智，不必猜测配对规则。

## 备选方案

- **双模式（资产单元视图 / 文件视图）**：老体验不丢，但两套模型与状态长期并存，新人分不清哪套是真的。
- **资产单元为主 + 未识别文件抽屉**：改动最小，但空目录与纯未识别目录依然消失，仍不是文件夹浏览器。
- **每格式独立页面**：灵活，但每个新格式都要写一套导航，列表里也无法统一表达「这个文件能干什么」。

---

## English summary

The library's primary object becomes the **file entry** — one real file, keyed by absolute path — but only **native
game binary resources** are catalogued: registry-listed extensions appear, while derived artifacts such as `.png`,
`.json` and `.txt` never do (only a count is reported). The directory tree mirrors the real filesystem with lazy children,
empty directories included, plus breadcrumbs and back/forward/up. Pairing is demoted to a derived "related files" relation that neither
groups the list nor controls visibility. A viewer panel on the right renders by the file's kind, and one file may expose
several viewers (animation = playback + structure tabs). The old scan contract (`scan_folder`, `AssetUnit`,
`enrich_counts`) is deleted rather than kept in parallel. Cost: a near-total rewrite of the scan layer, store and library
components. Benefit: visibility stops depending on parser progress, and a new format costs one registry row plus one viewer.
