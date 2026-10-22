# ADR-0018：格式能力下沉到 Format，统一文档树查看器

- **状态**：已接受（2026-10-22）
- **影响范围**：`src-tauri/crates/vw-core`（format/dto/game）、`src-tauri/crates/vw-formats`（doc）、`src-tauri`（doc 命令）、`src/lib/ipc.ts`、`src/components/viewers`
- **补充**：ADR-0013（格式注册表在 backend）；取代其中「viewers/thumbnail 由 Kind 决定」的部分

## 背景

ADR-0013 把 `viewers` 与 `has_thumb` 挂在 `Kind` 上。当格式从 3 种扩到 20+ 种时，
`Kind` 同时承担「粗分类」与「格式标识」两种职责：

- 若一格式一 Kind，前端 `Kind` 类型、过滤状态、图标分支全部随格式数量膨胀；
- 若保留 3 个 Kind，`ftx` 与 `mbs` 之外的格式无法声明自己的查看器与缩略图能力。

同时，每一种格式都需要一个「结构」视图；如果每个格式各写一个前端树组件，
新增格式的成本会落在 TypeScript 而不是 Rust。

## 决策

- **`Kind` 保持粗分类**，只用于图标、中文标签与分组：`texture / animation / scene / model / script /
  text / list / grid / binary`。扩展名级差异全部由 `Format` 声明。
- **`Format` 承载能力**：`ext / kind / label / viewers / has_thumb`。查看器顺序即 tab 顺序；
  `has_thumb` 只对 FTEX 为真。
- **注册表快照命令 `list_formats`** 是工具栏过滤项的唯一真值；前端不再硬编码格式或类别数组。
- **标签过滤按扩展名**：`formatFilter: string[]` 为空表示全部；列表与全局搜索都按 `FileEntry.ext` 过滤。
- **统一文档树**：后端把每种格式投影成同一套 arena：

  - 节点以数组下标为 id，子节点只存 id；
  - `open_doc` 返回根页（100 项），`doc_children(handle, node, offset, limit)` 分页取子树；
  - 上限 `MAX_NODES 100k` / `MAX_DEPTH 64` / `MAX_STRING 256` / `PAGE_MAX 500`；
  - 校验不变量以 `DocCheckDto` 返回，异常节点用 `mark` 标注，不 panic。

- **DTO 放在 `vw-formats::doc`**，不引入 `vw-formats → vw-core` 反向依赖；`vw-core` 继续只做文件系统与注册表。
- **文档缓存 `DocCache`**：键为 `path + tag + size + mtime`，容量 8 份或 96 MB；
  句柄为 `u64`，淘汰后返回 `HANDLE_EXPIRED`，前端重新打开而不是静默空白。
- **游戏档案**：有界目录名嗅探（根 + 2 级父目录 + 最多 256 项目录项），只决定默认 `mbs_tag`；
  用户手选 tag 的优先级高于档案默认值（`tagSource = "manual"`）。
- **第二批查看器**：`asb` 提供反汇编（`doc_asm`）与伪代码（`doc_pseudo`，表达式栈模拟 + label/goto）；
  `bbs` 提供布局示意图（`doc_layout`，bbox / 物件 / 雾区四边形 / 相机轨迹）；`fms` 提供文本表
  （`doc_text`，虚拟列表 + 子串过滤）。三者都复用同一套 Rust 解析结果，不引入前端格式知识。

## 权衡

代价：`Kind` 与 `Format` 两个概念并存，新增格式需要同时选择粗分类；文档树不是「无损 JSON」，
只投影结构、区间与样本，具体像素/网格仍由专用查看器负责。
收益：新增格式只需注册表一行 + 一个 Rust 投影函数；前端只有一个文档树组件；
过滤、图标、缩略图能力全部来自后端，不会出现两处判断不一致。

## 备选方案

- **一格式一 Kind**：前端过滤最直接，但类型与持久化状态随格式数量膨胀，且与 `Format` 重复。
- **每格式一个前端树组件**：可以定制字段呈现，但 N 种格式就是 N 个组件，新增格式的成本落在错误的一层。
- **文档树用嵌套 JSON**：一次 IPC 返回整棵树，简单但会把几 MB 的 asb/fmd 全部塞进消息，分页失去意义。

---

## English summary

`Kind` stays a **coarse category** (nine values) used only for icons, Chinese labels and grouping; all
extension-level capabilities move to `Format` (`ext / kind / label / viewers / has_thumb`). A `list_formats`
command becomes the single source of truth for the toolbar filter, and filtering is by extension. Every format
projects its structure into one **flat arena DTO** (`vw-formats::doc`), served through `open_doc` + paged
`doc_children` with hard node/depth/string/page limits; checks and warnings are data, never panics. The doc DTO
lives in `vw-formats` so no reverse dependency on `vw-core` is introduced. `DocCache` keys on path + tag + size +
mtime, keeps 8 entries or 96 MB, and reports `HANDLE_EXPIRED` after eviction. Game profiles are detected with a
bounded directory-name scan and only supply a default mbs tag; a manual tag choice always wins.
