# ADR-0016：全部 Rust 代码收进 `src-tauri`，并按「命令 / 服务 / 数据结构 / 配置」分层

- **状态**：已接受（2026-10-22）
- **影响范围**：仓库根布局、workspace 成员、`src-tauri/src` 模块划分、`tools/ksc_gen.py`、文档内路径

## 背景

Rust 代码原本分散在仓库根 `crates/` 与 `src-tauri/` 两处：

```
crates/{vw-core, vw-formats, vw-media, vw-gen}/    ← 库
src-tauri/src/{lib,main,commands,scene,state}.rs   ← 应用
```

问题有三：

1. **心智负担**：想知道"这个工具由哪些 Rust 代码组成"要看两个顶层目录，且 `crates/` 还被 `.gitignore` 忽略，从来没进过版本库。
2. **职责边界模糊**：`commands.rs` 同时放命令、DTO、缩略图解码调度；`scene.rs` 同时放 DTO 与解析逻辑；`state.rs` 同时放缓存、索引任务与解析缓存。
3. **格式扩展成本**：新增一种资源既要动 `vw-core`（注册表）又要动命令层，但两者位置相距很远，容易漏改。

## 决策

- **所有 Rust 代码位于 `src-tauri/` 之下**：
  - `src-tauri/src/` —— 应用本体；
  - `src-tauri/crates/{vw-core, vw-formats, vw-media, vw-gen}/` —— 按领域拆分的库 crate（仍是独立 crate，不是单 crate 内模块）。
  - workspace 成员改为 `src-tauri/crates/*`；路径依赖、`vendor/kaitai` 相对路径、`tools/ksc_gen.py` 输出目录同步调整。
- **应用本体按职能分层**：

  | 目录 / 文件 | 职责 | 不许出现 |
  | --- | --- | --- |
  | `src/command/` | Tauri 命令薄层：参数校验 → `spawn_blocking` → DTO | 业务算法、格式判断 |
  | `src/service/` | 业务服务：场景解析与组装、解析 LRU、索引构建 | `#[tauri::command]` |
  | `src/model/` | 跨进程数据结构（DTO）与 `serde` 属性 | 解析、IO |
  | `src/config.rs` | 路径与可调参数（缓存目录、解码并发） | 业务逻辑 |
  | `src/state.rs` | 把服务装进 Tauri 托管的 `AppState`（持有 + 转发） | 业务算法 |
  | `src/lib.rs` | 装配：插件、命令注册、缓存目录初始化 | 业务逻辑 |

- **命令按领域分文件**：`library`（目录/详情/缩略图/头部字节）、`index`（构建/状态/搜索）、`texture`（帧/缩略帧/原尺寸/图集）、`animation`（场景/结构/导出）、`cache`。
- **命名统一**：模块名用英文小写；命令名用 `动词_名词`（`list_dir`、`file_meta`、`search_files`）；DTO 以 `Dto` 结尾。

## 权衡

代价：一次大范围移动，所有文档与脚本里的路径都要跟着改；`src-tauri` 目录体积与编译产物集中在一处。
收益：Rust 只有一个人口；每层职责单一，新增格式时"注册表 + 命令 + 服务"各有明确落点；`crates/` 终于能进版本库。

## 备选方案

- **合并为单一 crate（`src-tauri/src/*` 内模块化）**：最彻底，但 `vw-formats` 依赖 `vw-gen` 的生成物与 `vendor/kaitai`，
  合并后无法再用 crate 边界阻止"扫描层意外解码"（`vw-core` 不依赖 `vw-formats` 这一约束会消失）。
- **保持 `crates/` 在仓库根，只调整模块划分**：改动小，但不满足"Rust 集中存放"的要求。
- **`src-tauri/crates/` 继续被忽略**：布局变了但代码仍不入库，问题 1 未解决。

---

## English summary

All Rust code now lives under `src-tauri/`: the application in `src-tauri/src/`, and the domain crates in
`src-tauri/crates/{vw-core,vw-formats,vw-media,vw-gen}` (kept as separate crates so the "scan layer cannot decode"
boundary survives). The application is layered by responsibility — `command/` (thin Tauri commands per domain),
`service/` (scene parsing/assembly, parse LRU, index building), `model/` (DTOs), `config.rs` (paths, tunables),
`state.rs` (Tauri-managed container), `lib.rs` (wiring). Workspace members, path dependencies, the `vendor/kaitai`
relative path and the ksc generator output directory were updated together, and `crates/` is no longer gitignored.
Cost: a wide move that touches docs and scripts. Benefit: one entry point for Rust, single-responsibility modules, and a
clear landing spot per layer when a new format is added.
