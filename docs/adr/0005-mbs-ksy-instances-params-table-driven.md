# ADR-0005：`vw_mbs_v77.ksy` 用 instances + params 表驱动建模分节表

- **状态**：已接受（2026-10-20）
- **影响范围**：`spec/ksy/vw_mbs_v77.ksy`、`crates/vw-gen` 生成物、`crates/vw-formats/src/mbs` 解析层

## 背景

MBS 不是顺序结构。它的读法是数据驱动的：

1. 文件头里每个分节表项给出 `sp`（该分节数据的绝对偏移）与 `count`；
2. 按固定步长 `sk` 读 `count` 条记录；
3. 记录之间靠**索引**互相引用（s4 引用 s0/s1/s2，s5 引用 s3，s6 引用 s4/s5/s8…）。

Kaitai 的 `seq` 无法表达这种布局，但 `instances` + 绝对 `pos` + `repeat-expr` 可以。
项目既有约定是「ksy 只描述结构，像素/编解码手写」（见 `vw_ftx.ksy` 的 doc 段），本 ADR 沿用该约定。

## 决策

- `spec/ksy/vw_mbs_v77.ksy` **必须存在**，并用：
  - `params` 承载平台差异（`tag`、各分节的 `sk` 步长与语义开关）；
  - 顶层 `instances` 读分节表头（`sp` / `count` / 步长）；
  - 每个分节一个 `instance`：`pos: sp`、`repeat-expr: count`、`size` 由类目尺寸决定。
- 分节表、blend 表、mix 表以 **Rust 侧数据表**（`const` 数组或 `include_str!` 解析）落地，
  与 ksy 的 params 一一对应；ksy 只描述**一条记录长什么样**，不硬编码平台值。
- 像素/图集相关解码（内嵌 FTEX → 图集）不进 ksy，走 `vw-formats` 既有 codec 层。
- ksy 由既有生成链（`tools/` 下的生成脚本）产出 `vw-gen` 结构体；生成物禁止手改。

## 权衡

代价：`instances` 的绝对 `pos` 会让 ksy 变得「读起来不像声明式」，且 `params` 增多后 ksy 本身也会变长。

收益：结构、偏移、步长三项都在单一文件里可读可 diff；生成的结构体与手写解析器共用同一份定义，不会漂移；
符合项目「ksy 描述结构」的既有惯例。

## 备选方案

- **ksy 只描述头部 + 表常量，数据解析全手写**：结构定义与解析逻辑分家，两处都要维护同一份偏移，易漂移。
- **纯手写、ksy 只当文档**：偏离项目惯例，也在用户点名要求之外。

---

## English summary

MBS reads are data-driven (absolute `sp`, `count`, fixed stride `sk`, index-based cross-references), which
`seq` cannot express. `vw_mbs_v77.ksy` must therefore use `params` for platform differences plus `instances`
with absolute `pos` and `repeat-expr` for each section. Platform constants (section/blend/mix tables) live as
Rust-side data tables kept in 1:1 correspondence with the ksy params; the ksy describes only *what one record
looks like*. Embedded-texture decoding stays in the existing codec layer. Generated structs are not hand-edited.
