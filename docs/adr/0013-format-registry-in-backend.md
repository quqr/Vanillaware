# ADR-0013：格式能力由后端注册表声明，前端只按查看器 id 路由

- **状态**：已接受（2026-10-21）
- **影响范围**：`crates/vw-core`（新 `format` 模块）、`src-tauri`（DTO 增字段）、`src/lib/ipc.ts`、`src/components/viewers`

## 背景

「后续会有任务添加更多的二进制资源解析」。若格式知识散落在前端（按扩展名判断图标、决定能不能预览/看结构），
每加一种格式就要同时改前端与后端，且两处判断迟早不一致；后端明明能解码，前端却不知道该怎么呈现。

## 决策

- 后端持有一份**格式注册表**，`Format Descriptor` 是唯一真值，声明：
  - `kind`：**只含原生游戏二进制资源的类别** —— `texture` / `animation` / `binary`（已确认是原生资源但尚无解析器）
  - `icon`：语义图标名（`texture` / `animation` / `binary`）
  - `viewers`：可用查看器 id 列表，如纹理 → `["atlas"]`，动画 → `["animation"]`，其余 → `["hex"]`
  - `has_thumb`：能否出缩略图
  - `label`：给人看的格式名（如 `FTEX`、`MBS`）
- **注册表即白名单**：只有登记在册的扩展名才算资源。`.png` / `.json` / `.txt` / `.quad` / `.rgba`
  这些要么是别的工具链的产物、要么是本项目的输出，一律**不收录**——目录列表直接过滤，只报
  `hiddenFiles` 数量。图片与文本查看器因此不存在，也不需要。
- 匹配方式：按扩展名（小写、不含点）查表；查不到即不收录，没有 `unknown` 类别。
  预留魔数嗅探接口，但本次不实现——不要为尚未存在的需求写代码。
- 前端只认识**查看器 id → 组件**的映射；遇到未注册的 id 退化为 `hex` 查看器并显示一行提示。
- 缩略图统一走后端 `file_thumb(path)`，内部按 `kind` 分派（目前只有纹理解首帧），
  复用现有磁盘缓存与 `SCHEMA` 失效机制。前端不知道格式细节。

## 权衡

代价：每种新格式必须有后端一条注册表 + 一个解码命令，前端不能「自己判断先跑起来」。
收益：能力声明一致、可查询（列表就能显示「能干什么」）、可测试（注册表是纯数据）。

## 备选方案

- **前端按扩展名判断**：改动小，但解码在后端、判断在前端，契约分裂两处。
- **每格式独立页面**：把格式知识推进路由层，列表里无法统一表达能力。
- **按魔数嗅探优先**：更准，但需要读取每文件头部，列表滚动成本高；且现有格式扩展名可靠。

---

## English summary

A backend **format registry** is the single source of truth for what a file is and what can be done with it: `kind`
(native binary resources only — texture, animation, binary), `icon`, `viewers`, `has_thumb`, `label`, matched by
lowercased extension (magic-number sniffing reserved, not built). The registry doubles as a **whitelist**: derived
artifacts such as `.png`, `.json`, `.txt` and `.quad` are not catalogued and are filtered out of listings, with only a
count reported. The front end maps viewer ids to components and falls back to a hex viewer for unknown ids. Thumbnails go
through one backend command dispatched by kind, reusing the existing disk cache. Cost: every new format needs a registry
row plus a decode command. Benefit: capabilities are declared once, queryable per file, and testable as plain data.
