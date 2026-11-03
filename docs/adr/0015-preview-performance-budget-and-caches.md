# ADR-0015：预览与浏览的性能预算与三级缓存

- **状态**：已接受（2026-10-22）
- **影响范围**：`Cargo.toml`（profile）、`src-tauri/crates/vw-media`、`src-tauri/crates/vw-core`、`src-tauri/src/{state,config}.rs`、`src-tauri/src/command/*`、`src-tauri/src/service/*`、`src/lib/{thumbCache,pixiScene}.ts`、`src/components/{viewers,library}`

## 背景

"索引、查看、预览都慢"。实测（同一份 4096² / 56MB 的 `Map_effect.ftx`）：

| 步骤 | debug | release |
| --- | --- | --- |
| probe 4 帧 | 65 ms | 32 ms |
| 解码 4096² BC7 | **4.0 s** | 525 ms |
| 缩略图 PNG | **2.5 s** | 125 ms |
| 原尺寸 PNG | **7.9 s** | 97 ms |

同时定位到四个非构建因素：

1. **图集误用缩略图管线**：`get_atlas_png` 走 `render_thumb_png`，4096² 被压成 256² 再拿去 UV 采样——既糊又慢，属于正确性 bug。
2. **重复解析**：动画查看器的「播放」与「结构」两个 tab 各跑一次 `parse_file + convert`（5MB 文件约 205+52 ms）。
3. **WebGL 上下文每次重建**：`ScenePlayer` 的场景变化会销毁并重建 `Application`。
4. **列表用缩略图**：20px 的行内图标会触发整张 4096² 图集的解码。
5. 索引里 `WalkDir::sort_by_file_name()` 让每个目录先读完再遍历（索引本身只需 8.7ms/722 文件）。

## 决策

- **开发构建开优化**：`[profile.dev] opt-level = 1` + `[profile.dev.package."*"] opt-level = 3`。
  自研 crate 保留可单步的优化级别，解码/编码链（依赖）拉满。性能基准以 release 数量级为准。
- **缓存分两档，各自版本化**：
  - **缩略图**：最长边 128px、PNG 快速压缩档（box 采样），只给列表/网格；
  - **帧图**：原尺寸 PNG，只给查看器与图集采样。
  `SCHEMA` 统一失效（当前 5）。
- **解析 LRU**：按 `(path, tag, size, mtime)` 缓存最近 4 份 `LoadedMbs`，
  `preview_scene` / `convert_mbs` / `export_quad` 共用；文件改动即失效。
- **图集预取**：进入动画查看器后按场景的 `texId` 顺序串行预取，首帧纹理先到。
- **渲染器复用**：`PixiRenderer` 只在挂载时创建一次，切文件只改数据与相机。
- **列表缩略图延迟串行**：滚动停止 300ms 后才开始，一次一张；网格视图保持立即加载。
- **索引不排序**：只服务搜索，顺序无意义。

## 权衡

代价：dev 首次全量编译变慢（一次性）；缩略图 128px 在超大卡片上会略糊（当前最大卡片 112px，够用）；
解析缓存占几 MB～几十 MB 内存；列表图标会比缩略图晚一拍出现。

收益：同一操作从"秒级等待"降到"百毫秒级"；重复打开同一文件零解析成本；
图集恢复无损，预览清晰度回到应有水平。

## 备选方案

- **只依赖 release**：开发期每次看图都要等 4s，不可接受。
- **原尺寸 RGBA 过 IPC**：省掉编码，但 4096² 一张 64MB，内存与拷贝成本更高。
- **图集降到 2048 上限**：传输小，但放大四倍时文字/边缘糊。
- **列表直接不出缩略图**：最快，但放弃了"列表也能扫一眼"的价值。

---

## English summary

Perceived slowness had one dominant cause — `tauri dev` runs an unoptimized build, where BC7 decode and PNG
encode are 7–80× slower than release — plus four real defects: the atlas was squeezed through the 256px thumbnail
pipeline, the play and structure tabs each re-parsed the same `.mbs`, the Pixi `Application` was rebuilt per file, and
20px list icons triggered full 4096² decodes. Decisions: enable optimization in dev (own crates `opt-level=1`,
dependencies `3`); split the cache into ≤128px thumbnails (fast PNG compression) and full-resolution frame images;
cache the last four parses keyed by path+tag+size+mtime and share them across play/structure/export; prefetch all scene
`texId`s on open; create the renderer once and swap data; defer list thumbnails to 300ms-after-scroll and run them
serially, while the grid stays immediate; drop directory sorting from the index. Cost: slower first dev build, slightly
soft 128px thumbnails on large cards, tens of MB of cache memory. Benefit: interactions drop from seconds to hundreds of
milliseconds, repeat opens become free, and preview sharpness is restored.
