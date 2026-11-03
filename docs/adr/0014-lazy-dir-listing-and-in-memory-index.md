# ADR-0014：目录读取懒加载，全局搜索走纯内存索引

- **状态**：已接受（2026-10-21）
- **影响范围**：`src-tauri/crates/vw-core`（新 `fslist` / `index`）、`src-tauri`（后台线程与状态）、`src/store/libraryStore.ts`

## 背景

改成文件夹浏览后，「打开根就把全树扫一遍」这条路走不通：语料 10,234 文件尚可，但同类工具面对完整解包
（十万级文件）时，每次打开都要等整棵树，且未识别格式增多后元信息读取成本线性上升。

同时用户要求搜索能在全库范围可用，而不是只能筛当前目录。

## 决策

- **目录读取懒加载**：打开根只读这一层；展开子目录时再 `read_dir`。列表永远来自实时读取，
  不以索引结果冒充。
- **后台索引只做元信息**：打开根后后台线程递归收集（路径、名称、扩展名、类别、大小、mtime），
  不读文件内容、不做解码、不做缩略图。
- **索引纯内存**：进程内 `Vec`/映射，换根或手动刷新即整份丢弃重建。不落盘，因此没有版本号、
  失效判定与清理，也不会与缩略图磁盘缓存产生第二套失效机制。
- **搜索带作用域**：默认「本目录」（对当前列表过滤，不依赖索引）；切到「全部」才走索引，
  结果行显示所在目录。作用域是显式开关，不做自动跳转。
- **索引状态可观测**：进行中/就绪/失败 + 已收集条数，通过 `index_status` 暴露给状态栏。

## 权衡

代价：全局搜索在索引完成前不完整（UI 必须显示进度，不能用空结果冒充「没有匹配」）；
换根后需重新建索引。
收益：打开大目录是常数成本；不引入磁盘缓存失效这一整类 bug；内存索引十万条路径约几十 MB，可接受。

## 备选方案

- **全量递归扫描一次**（原实现）：实现最简，但每次打开都要整树扫描。
- **索引落盘**：二次打开更快，但要版本号、失效判定、迁移与清理。
- **只索引已访问目录**：最省资源，但搜索结果不完整且难以让用户察觉。

---

## English summary

Directories are read lazily: opening a root reads one level, expanding a child reads that level, and listings always come
from a live `read_dir` rather than the index. A background thread builds a **metadata-only, in-memory** index of the whole
tree for global search — no file contents, no thumbnails, no on-disk cache, so there is no second invalidation scheme.
Search has an explicit scope: current directory (list filter) by default, whole tree via the index on demand, with index
progress surfaced in the status bar. Cost: global results are incomplete until indexing finishes, and reopening a root
rebuilds the index. Benefit: constant-cost navigation, no cache-staleness bugs, tens of MB for 100k paths.
