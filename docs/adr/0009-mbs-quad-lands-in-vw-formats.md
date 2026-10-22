# ADR-0009：MBS / V55 / Quad 代码并入 `vw-formats` 的 `mbs` 模块

- **状态**：已接受（2026-10-20）
- **影响范围**：`crates/vw-formats`、`crates/vw-core`、Tauri 命令层

## 背景

候选落点有三个：新建 `vw-quad` crate、并入 `vw-formats`、放进 `vw-core`。
`vw-core` 是扫描 / 配对 / 资产层，放格式解析会让它既管文件系统又管二进制格式。
新建 crate 边界干净，但会引入一个只服务单个数据族的 crate，且 `vw-gen` 结构体已在 `vw-formats` 侧被消费。

## 决策

- 格式解析（MBS 分节表 → V55 模型 → Quad 模型、图集解码、校验器）**并入 `vw-formats`**，
  置于与本文件同级的 `mbs/` 模块（与既有 `ftx/` 并列）。
- `vw-gen` 结构体仍由 `vw-formats` 消费；ksy 生成链不变。
- `vw-core` 只新增「资产单元 → mbs 路径 → 预览会话」的编排，不承载格式逻辑。
- Tauri 命令层新增预览与转换所需命令；前端只经 `ipc.ts` 调用，不直接触达 crate。

## 权衡

代价：`vw-formats` 从「只做纹理」变为「纹理 + 动画」，职责面扩大一格。

收益：避免一个单数据族的 crate；生成物与消费方同处一个 crate，依赖方向简单。

## 备选方案

- **新建 crate `vw-quad`**：边界最干净，但为本轮唯一数据族付一个 crate 的固定成本，且 `vw-gen` 依赖要跨 crate。
- **放进 `vw-core`**：会混淆「文件系统/资产层」与「二进制格式层」，且 `vw-core` 已依赖 `vw-formats`，方向更乱。

---

## English summary

MBS parsing (section table → V55 → Quad), atlas decoding and the verifier go into `vw-formats`, in a new
`mbs/` module alongside the existing `ftx/`. `vw-gen` structs continue to be consumed there, so the ksy
generation chain is unchanged. `vw-core` only gains orchestration (asset unit → mbs path → preview session);
it keeps no format logic. New Tauri commands expose preview/convert to the front end via `ipc.ts`. A dedicated
`vw-quad` crate was rejected as fixed cost for a single data family.
