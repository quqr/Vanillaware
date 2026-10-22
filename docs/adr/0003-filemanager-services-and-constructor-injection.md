# ADR-0003：FileManager 拆为三个服务，并成为首个使用构造注入的页面

- **状态**：已接受（2026-10-20）
- **影响范围**：File Manager 的 ViewModel / code-behind / 依赖获取方式

## 背景

`FileManagerViewModel` 已接近 800 行，混合了五类职责：目录与文件选择、扫描与配对、FTX/MBS 解析、缩略图生成、导出、UI 状态。它内部 `new` 出 `QuadFileLoader` 与 `ThumbnailGenerator`，又通过 `Instances.ServiceProvider` 反查 `PlayerViewModel` 与 `RootViewModel`；页面 code-behind 再从 DI 容器取 ViewModel，并用手写 C# 拼装 `ContextMenu`。

按本次改造（虚拟化、磁盘缓存、筛选/排序、导出语义），职责只会继续膨胀。同时配对规则（ADR-0001）与缓存键（ADR-0002）都是「写错了会静默出错」的纯逻辑，目前完全没有测试覆盖，而仓库已有 xunit 工程 `Tests/`。

## 决策

拆出三个服务，均由 ViewModel 构造函数注入：

| 服务 | 职责 | 是否可单测 |
| --- | --- | --- |
| `AssetScanner` | 递归扫描、按 ADR-0001 配对、产出资产单元 | 是（纯路径逻辑） |
| `ThumbnailCache` | 按需解码、写/读磁盘缓存与 sidecar、失效判定（ADR-0002） | 是（缓存键与失效） |
| `AssetExporter` | 批量/单个导出、为 Previewer 生成临时文件的完整生命周期（含清理） | 部分 |

ViewModel 保留：筛选/搜索/排序、选择状态、命令、以及面向视图的展示属性。

同时：

- code-behind 中的 `ContextMenu` 构造搬进 XAML，菜单项绑定命令。
- 移除 ViewModel 内对 `Instances.ServiceProvider` 的反查，`PlayerViewModel` / `RootViewModel` 改为构造参数。
- 页面 View 仍从 DI 容器解析 ViewModel。**这是 Avalonia 的限制**：XAML 以无参构造实例化 `UserControl`，视图层无法直接构造注入。本 ADR 只要求「ViewModel 的依赖是显式的」，不承诺「View 的依赖是显式的」。

## 权衡

- 本页面会成为仓库里第一个不使用 `Instances` 反查的页面，与其余页面不一致。接受这一不一致，把它当作试点；若验证有效，再按页面逐步推广（见 `docs/architecture-analysis.md` 候选 4）。
- 改动面覆盖 VM、code-behind、XAML 三层，无法与行为修复完全分开，因此必须按批次提交（配对与移除语义 → 虚拟化与懒加载 → 缓存 → 搜索/筛选/导出）。
- 收益：资产单元的产出与缓存键成为可测试单元；VM 从「什么都做」退化为「只描述界面状态」。

## 备选方案

- **只改行为、不拆结构**：diff 更小，但 VM 会突破 1100 行，且配对与缓存逻辑仍无法测试。
- **一次性重构整个应用的服务定位模式**：超出本页面范围，review 成本过高，应另开任务。
