# 0021 — 内容视图与媒体播放：结构之外还要看得见「是什么」

> **注意**：本文中「`.cpk` 不投入」的结论已被 ADR-0025 取代（CPK 现为可挂载的虚拟子目录，类别 `Kind::Archive`）。正文保留原样，仅此标注。

## 状态

已采纳（2026-10-28）。

## 背景

到 ADR-0020 为止，注册表里每个格式都能逐字节闭合，但闭合只证明「读得完」，
不证明「看得懂」。文档树回答的是「文件里有哪些字段」，用户打开 `.drg` 看到的是
`+0x38 u32 ×96`，打开 `.acb` 看到的是表头偏移——**结构正确，内容缺席**。

用户在这一点上明确要求：可视化内容，不只是结构。

## 决策

### 1. 在文档树之外另立「内容视图」层

`vw-formats/src/content.rs` 把字节投影成一种**中性的内容模型**，只有四样东西：

| 形态 | 结构 | 用途 |
|---|---|---|
| 表 | `Section { title, columns, rows, total }` | 调试库命令表、UCB 记录、ACF 配置表 |
| 灰度位图 | `GrayMap { width, height, values, legend }` | 大地图、掩码/位域图 |
| 折线 | `Series { label, points, note }` | `.nnd` 的数值序列、`.drg` 的 f32 体 |
| 文本 | `Vec<String>` | `.scb` 还原的源码、`.inc` 枚举表 |

`vw-media/src/content.rs` 只负责把位图编成 PNG 并内联成 data URL，别的原样透传。
前端 `ContentViewer` 一份组件同时渲染四种形态，不为每个格式写一个查看器。

**为什么不做成「一格式一查看器」**：这些格式的内容形态高度重合（表 / 位图 / 折线 / 文本），
一格式一组件会把「怎么显示表格」复制十几遍，且新增格式必然要动前端。

### 2. 音频：后端解 HCA，前端解 Opus

UO 的语音 97% 是 **CRI Opus（`lopus`）**，不是 HCA；只有 1556 条 cue 是 HCA。
两种编码走两条路：

- **HCA**：`cridecoder`（纯 Rust，MIT）在 `vw-media/src/audio.rs` 解成 i16 PCM，
  后端以二进制响应回交错样本；
- **lopus**：`0x80000001` 基本块 + `0x80000004` 数据块，每帧定长（248 B），
  前 8 字节帧头，其后 240 B 是裸 Opus 包。后端只做拆包，前端用 WebCodecs
  `AudioDecoder` 解码，**不引入第二个解码库**。

`audio_clip_info` 与 `audio_clip_data` 分成两个命令：元信息走 JSON，
载荷走二进制，几十兆 PCM 不进 JSON。

前端解码上限 300 秒（语料最长的 BGM 是 107 秒），防止异常长轨道把浮点数组吃光。

### 3. 影片：后端建帧索引，前端 WebCodecs 出画

`vw-media/src/video.rs` 不整段读入，而是按 USM 块链流式扫描：
`@SFV` 块内以 AUD（`00 00 00 01 09`）切帧，记录每帧的流内偏移与关键帧标记，
顺便从首个 SPS 读出宽高/profile/level，从块头读帧率。

前端按 GOP 取字节（`usm_video_gop`），Annex-B 转 AVCC（`avcC` 由 SPS/PPS 拼），
`VideoDecoder` 解成 `ImageBitmap` 画到 canvas。WebCodecs 不可用时降级成时间轴统计。

**为什么自己写索引而不直接用 `cridecoder` 的 USM 提取**：它一次把整条视频流吐进内存
（`PV#1_Master` 是 76 MB），看片头也要等全解；按 GOP 取才能开箱即用。

## 后果

- 新增格式只要在 `content.rs` 里写一个投影函数，前端零改动。
- 音频/影片解不出来时，同一个文件仍可切到「结构」与「十六进制」，不会白屏。
- `.cpk` 维持 `Kind::Binary`（用户明确不投入），它是这套内容层里唯一没有内容视图的格式。
- 位图纵轴/横轴过长的掩码流会先折叠（`.nnd` 3220 组、`.vs2` 17280 条 → 768 列），
  否则出图是一条细线。
