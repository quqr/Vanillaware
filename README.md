# Vanillaware 资源浏览器

一个只读的本地资源浏览器，用于查看 Vanillaware 两款游戏的原生二进制资源：
纹理、动画、场景、模型、脚本、文本、列表、网格、音频、影片，以及可挂载的 CPK 归档。

它不是通用解包器：只显示登记在格式注册表里的原生游戏资源，其余文件只计入「未收录」数量。

## 支持范围

| 项目 | 状态 |
| --- | --- |
| **UNICORN OVERLORD**（Switch） | 已验证 |
| **13 Sentinels: Aegis Rim**（Switch，内部代号 ROBO） | 已验证 |
| PS4 / PS5 等其它平台档案 | 可以打开，但**未验证**；选中非 `swi_*` 的 `tag` 时界面会挂「未经验证」徽标 |

## 功能

- **列视图浏览**：Finder 风格的分栏视图，右侧固定预览栏；单击看元信息，双击装载查看器。
- **CPK 归档**：`打开归档` 选中一个 `.cpk` 后即可像目录一样进入，按需解包单条资源；
  自洽格式零写盘，需要同目录邻居的格式（`.mbs`+`.ftx`、`.acb`+`.awb`、`.usm` 等）按目录投影到缓存盘。
  归档概览面板给出条目数、目录数、压缩占比、表偏移与加密情况。
- **查看器**：图集、骨骼动画播放、文档树、内容视图（表 / 位图 / 折线 / 文本）、
  反汇编与伪代码、对话、音频（HCA 后端解码 + lopus 交前端 WebCodecs）、
  USM 影片（按 GOP 取流）、3D 网格、场景图、引用关系图、十六进制。
- **全局搜索**：后台建立的目录索引，覆盖磁盘目录与已挂载归档内部条目。
- **格式过滤**：按扩展名筛选当前列。

## 开发

前端 Tauri v2 + React 19 + Tailwind 4 + daisyUI 5，后端 Rust。

```bash
pnpm install
pnpm tauri dev          # 开发
pnpm build              # 前端类型检查 + 构建
```

## 验收

```bash
# Rust
cargo test -p archive                                  # CPK 读取器
cargo test -p archive --test corpus -- --ignored       # 四个真实 CPK 的字节级闭合
cargo test -p vfs                                      # 挂载层
cargo test -p vfs --test corpus -- --ignored           # 真实归档的列举与索引
cargo test -p catalog -p formats -p media              # 格式与媒体回归
cargo test --test archive_mount                        # 归档到查看器链路的端到端
cargo run -p archive --example cpk_probe -- <x.cpk>    # 归档诊断，有 FAIL 非零退出

# 前端
pnpm check:lang                                        # 代码里不得出现中文
npx tsc --noEmit
```

语料路径可用 `CPK_CORPUS`、`CORPUS_13`、`CRIFS_ASSETS` 等环境变量覆盖，
默认值写在对应 `tests/*.rs` 的常量里。

## 结构

```
src-tauri/
├── src/                 应用层：command（薄命令层）/ service（业务与缓存）/ model（DTO）
└── crates/
    ├── catalog          目录列举、格式注册表、索引、游戏档案（刻意不依赖 formats）
    ├── formats          格式语义与投影（文档树、内容视图、纹理解码、反汇编…）
    ├── archive          CRI CPK 读取器（容器 / @UTF 表 / TOC / CRILAYLA）
    ├── vfs              归档挂载层：一个路径同时寻址磁盘与归档
    ├── media            PNG 缓存、音频、影片、内容视图的媒体包装
    └── schema           Kaitai 生成的结构定义
src/                     前端：views / components（columns、library、viewers）/ store / lib / i18n
```

设计决策见 `docs/adr/`，领域术语见 `CONTEXT.md`。
