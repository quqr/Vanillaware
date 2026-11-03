# CONTEXT — Vanillaware 工具链领域词汇表

本文件是本仓库的**唯一术语权威**（glossary）。任何 issue、ADR、测试名、重构提案在命名领域概念时必须使用此处定义的词，不得另造同义语。

This file is the repository's authoritative glossary. When naming a domain concept in issues, ADRs, test names, or refactor proposals, use the term defined here. Do not invent synonyms.

> 维护方式：术语在决策真正落地时才追加，不预先臆造。中英并列，中文为准。
> Maintenance: terms are appended lazily when a decision lands. Chinese is normative.

---

## 支持范围 / Support scope

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **支持范围 / Support scope** | 本工具只验证过 **UNICORN OVERLORD** 与 **13 Sentinels: Aegis Rim**（内部代号 ROBO）的 **Switch 版**。其它平台（`ps4_*` / `ps5_*`）的档案可以打开，但结果未经验证，界面在 `tag` 非 `swi_*` 时挂「未经验证」徽标。 | Only the **Switch** releases of **UNICORN OVERLORD** and **13 Sentinels: Aegis Rim** (internal codename ROBO) are verified. Other platforms open but are unverified; the UI shows an "unverified" badge whenever `tag` is not `swi_*`. |

## 资产层 / Asset layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **文件条目 / File Entry** | 库里的最小展示单位：磁盘上的一个真实文件。以绝对路径为键。 | The smallest unit shown in the library: one real file on disk, keyed by absolute path. |
| **目录列表 / Directory Listing** | 某个目录的即时读取结果，含子目录与文件条目。列表以此为准，索引不能替代它。 | The live read of one directory: child dirs plus file entries. Listings are authoritative; the index never replaces them. |
| **格式描述符 / Format Descriptor** | 一种文件格式的能力声明：类别、图标、可用查看器、有无缩略图。后端是唯一真值。 | Capability declaration for a file format: kind, icon, available viewers, thumbnail support. Backend is the single source of truth. |
| **类别 / Kind** | 格式的粗分类，**只含原生游戏二进制资源**：纹理、动画、场景、模型、脚本、文本、列表、网格、二进制。图片与文本文件不属于本工具的收录范围；扩展名级差异由「格式」声明。 | Coarse class of **native game binary resources only**: texture, animation, scene, model, script, text, list, grid, binary. Image and text files are out of scope; extension-level differences live in Format. |
| **格式 / Format** | 扩展名级的能力声明：`ext / kind / label / viewers / has_thumb`。查看器与缩略图能力跟随格式，不跟随类别。见 ADR-0018。 | Extension-level capability declaration: `ext / kind / label / viewers / has_thumb`. Viewer and thumbnail capabilities follow the format, not the kind. See ADR-0018. |
| **查看器 / Viewer** | 针对某格式打开的呈现组件。同一文件可挂多个查看器（如动画的「播放」「结构」与「十六进制」）。 | A presentation component opened for a format. One file may have several viewers (e.g. animation playback, structure, hex). |
| **值流 / Value Stream** | `.bst` 的正文：1 字节类型标签 + 定长载荷，`0x34` 收尾一条记录。与 `.osb` 族的节点树是两套写法。见 ADR-0023。 | The body of a `.bst`: a byte tag plus a fixed payload, `0x34` closing a record — a different writer from the `.osb` node tree. See ADR-0023. |
| **碰撞板 / Plate** | 战斗关卡里一块可站立的四边形：id、中心点、角点索引。 | A walkable quad in a battle stage: id, centre, corner indices. |
| **bundle 容器 / Bundle** | 13 Sentinels 的场景族容器：`.osb`（舞台）/ `.oeb`（站位）/ `.orb`（轮次）共用「资源路径池 + 节点树」布局，节点 = 16 字节头 + body（caption/label/group）。见 ADR-0022。 | The 13 Sentinels scene-family container: `.osb` / `.oeb` / `.orb` share "resource pool + node tree", a node being a 16-byte header plus body (caption/label/group). See ADR-0022. |
| **内容视图 / Content View** | 文档树之外的一层投影：把字节变成表 / 位图 / 折线 / 文本，回答「文件里是什么」而不是「有哪些字段」。见 ADR-0021。 | A projection layer beside the doc tree: bytes become tables, bitmaps, series or text — what the file *is*, not which fields it has. See ADR-0021. |
| **音频轨道 / Audio Track** | `.acb` 里的一条 cue（名字来自 cue 表）或 `.awb` 里的一条 AFS2 条目。载荷可能在同目录的外部 AWB 里。 | One cue in an `.acb` (named by the cue table) or one AFS2 entry in an `.awb`. Payload may live in a sibling AWB. |
| **片段 / Clip** | 一条轨道解码后的结果：HCA 已完成解码（i16 PCM），lopus 只拆出裸 Opus 包，交给前端 WebCodecs。 | The decoded result of one track: HCA arrives as i16 PCM, lopus as bare Opus packets for the WebView's WebCodecs. |
| **帧索引 / Frame Index** | `.usm` 按 AUD 切出的帧表：每帧的流内偏移、大小、是否关键帧。按 GOP 取字节，不整段读入。 | The frame table of a `.usm`, cut on AUD: in-stream offset, size, keyframe flag. Bytes are fetched per GOP, never whole-file. |
| **GOP** | 关键帧到下一个关键帧之间的帧序列，是影片按需解码的最小单位。 | Keyframe to the next keyframe — the smallest unit fetched for on-demand decoding. |
| **关联文件 / Related Files** | 同目录下同基础名的其他文件（如 `.mbs` 与 `.ftx`）。派生关系，不决定列表分组，也不决定可见性。 | Other files in the same directory sharing a base name. A derived relation — it neither groups the list nor decides visibility. |
| **目录索引 / Directory Index** | 打开根目录后由后台建立的元信息快照，只服务全局搜索；不做缩略图、不做解码。 | A metadata snapshot built in the background after opening a root. Serves global search only: no thumbnails, no decoding. |
| **缩略图 / Thumbnail** | 列表与网格用的降采样图（最长边 128px），只为「认出是哪一个」服务，不用于采样。 | Downscaled image for list/grid (128px longest edge). For recognition only, never sampled. |
| **帧图 / Frame Image** | 查看器与图集采样用的原尺寸图。与缩略图分开缓存，绝不复用缩小版。 | Full-resolution image for viewers and atlas sampling. Cached separately from thumbnails. |
| **解析缓存 / Parse Cache** | 最近解析过的 `.mbs`（Quad + 图集元信息），按「路径 + tag + 大小 + mtime」命中；播放与结构共用。 | The most recent parsed `.mbs` results, keyed by path + tag + size + mtime, shared by playback and structure views. |
| ~~**资产单元 / Asset Unit**~~ | 已废弃，由「文件条目 + 关联文件」取代。 | Deprecated; replaced by File Entry + Related Files. |

## 格式层 / Format layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **纹理文件 / Texture file** | `.ftx` / `.ftp` 等 FTEX 容器，装的是压缩像素帧。类别 = 纹理。 | An FTEX container (`.ftx`, `.ftp`) holding compressed pixel frames. Kind = texture. |
| **动画文件 / Animation file** | `.mbs` / `.mbp` 等族文件，装的是骨骼-四边形动画数据。类别 = 动画。 | A family file (`.mbs`, `.mbp`) holding skeleton/quad animation data. Kind = animation. |
| **音频类别 / Audio kind** | CRI ADX2 的 `.acb`（cue sheet）/ `.awb`（AFS2 wave bank）/ `.acf`（配置表）。表结构按 `@UTF` 逐列解码，载荷（HCA）按字节保留。 | CRI ADX2 containers: `.acb` cue sheet, `.awb` AFS2 wave bank, `.acf` config. Tables decoded column-by-column; HCA payload kept as bytes. |
| **影片类别 / Video kind** | `.usm`（CRI Sofdec2）容器；块链走到 EOF，视频/音频载荷不解码。 | A `.usm` (CRI Sofdec2) container; the chunk chain is walked to EOF, payloads are not decoded. |
| **归档类别 / Archive kind** | 可挂载的容器格式，目前只有 CRI `.cpk`。它不是一种「资源」，而是一个可以进去的目录。见 ADR-0025。 | A mountable container format, currently only CRI `.cpk`. Not a resource but a directory you can enter. See ADR-0025. |
| **收录 / Catalogued** | 只有格式注册表里登记的原生资源才被收录。`.png`、`.json`、`.txt`、`.quad` 等派生产物在目录列表里被过滤掉，只报数量。 | Only registry-listed native resources are catalogued. Derived artifacts (`.png`, `.json`, `.txt`, `.quad`) are filtered out of listings; only a count is reported. |

## 归档层 / Archive layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **归档 / Archive** | 一个 CRI `.cpk` 文件：16 字节表容器 + `@UTF` 头表（按列名给出 `TocOffset` / `ContentOffset`）+ `@UTF` TOC（`DirName` / `FileName` / `FileSize` / `ExtractSize` / `FileOffset` / `UserString`）。头表与 TOC 都可能被 XOR 加密。见 ADR-0025。 | A CRI `.cpk`: a 16-byte table container, a `@UTF` header table naming `TocOffset` / `ContentOffset`, and a `@UTF` TOC with dir name, file name, size, extract size and offset. Header and TOC may be XOR-encrypted. See ADR-0025. |
| **挂载点 / Mount point** | 磁盘上那个 `.cpk` 文件在界面里的身份：它不再是一个文件条目，而是一个可进入的目录项（`kind = archive`）。 | The `.cpk` file's identity in the UI: no longer a file entry but an enterable directory item (`kind = archive`). |
| **虚拟子目录 / Virtual subdirectory** | 归档内部的目录。`<cpk 路径>\<归档内路径>` 就是一条普通路径，命令层不区分磁盘与归档。 | A directory inside an archive. `<cpk path>\<inner path>` is an ordinary path; the command layer cannot tell disk from archive. |
| **内存优先 / Memory-first** | 归档内单文件自洽的格式从内存字节解析，磁盘零写入；只有需要同目录邻居或单条超过 32 MB 时才落盘。 | Self-contained formats inside an archive are parsed from memory with no disk write; only formats needing siblings, or entries over 32 MB, are spooled to disk. |
| **CRILAYLA** | CRI 的压缩格式：头 16 字节（小端）+ 反向位流 + LZ77 回引，末尾 0x100 字节原样前缀。 | CRI's compression: a 16-byte little-endian header, a backwards bitstream with LZ77 back references, and a verbatim 0x100-byte prefix at the tail. |

## 世界层 / World layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **世界栅格 / World grid** | `Field.ucb`：把整个世界切成 2199 × 1311 个格子，每格记「属于哪片区域、属于哪类地貌」。它不含像素。 | `Field.ucb`: the world cut into a 2199 × 1311 grid, each cell naming the area it belongs to and a terrain class. It carries no pixels. |
| **区域号 / Area id** | 一个格子属于哪片战场区域。区域与 `Field.ufb` 的区域掩码一一对应。 | Which battlefield area a cell belongs to. Areas correspond one-to-one with the area masks in `Field.ufb`. |
| **类别码 / Class code** | 与区域号并列的另一个维度：地貌类别。含义尚未还原。 | A dimension beside the area id: the terrain class. Its meaning is not yet recovered. |
| **摆放记录 / Placement record** | `Field.ufb` 按 138 × 105 瓦片网格存放的一组图层，描述「这一小块世界摆了些什么」。**一个记录的图层可以来自好几个放置集**，所以它不能直接当作一张图来画。 | A set of layers in `Field.ufb` on a 138 × 105 tile grid: what is placed in that patch of world. **One record's layers may come from several placement sets**, so a record is not a picture. |
| **战场图 / Battlefield map** | 一组共享 `ACxx` 前缀的放置集，合起来构成一张横向全景（`AC00`..`AC10`），宽约六万世界单位、高只有几百。见 ADR-0027。 | The placement sets sharing an `ACxx` prefix, forming one horizontal panorama (`AC00`..`AC10`) some sixty thousand world units wide and only a few hundred tall. See ADR-0027. |
| **放置集 / Placement set** | 一个 `.MPD` 家族，即 `ACxx_yy`（`AC00_00P`、`AC00_02P`、`AC00_00F2` …），共 25 个。查看器默认一次只画一个（最好看），整族叠加（scope 写 `AC00`）也成立，只是云层会挡住地面。`F` 是远景细条，`P` 是近景层。 | One `.MPD` family, `ACxx_yy` (`AC00_00P`, `AC00_02P`, `AC00_00F2`, …), 25 in total. The viewer draws one at a time by default because that reads best; stacking a whole family (scope `AC00`) also works, it just lets the cloud layers cover the ground. `F` marks a thin far strip, `P` a near plane. |
| **区域名 / Area name** | 掩码 `+0x36` 指向 `Field.ufb` 末尾 179 条区域名（`戦場_Bridge_C01`、`Z_1通常_川CB11`、`AbyssMist`、`Elf_Barrier` …）。命名是「天气/章号_地形+编号」，`-1` 表示无名。 | Mask field `+0x36` indexes the 179 area names at the end of `Field.ufb` (`戦場_Bridge_C01`, `Z_1通常_川CB11`, `AbyssMist`, `Elf_Barrier`, …). The pattern is "weather/chapter_terrain+number"; `-1` means unnamed. |
| **F 记录 / F record** | `Field.ufb` 尾部的 11 条 `0x18`：`f32 x0, z0, x1, z1` 世界矩形 + 4 × u16 标签。它们占「模型对象」槽位（`modelObjects = 模型层 + F 记录`），但没有 pair / panel 字段，所以不参与绘制。 | The 11 `0x18` records after the masks: an `f32 x0, z0, x1, z1` world rectangle plus four u16 tags. They occupy model-object slots (`modelObjects = model layers + F records`) but carry no pair or panel, so nothing is drawn from them. |
| **道具名 / Prop name** | 图层 `+0x3e` 指向 337 条道具名（`Town_C18_1`、`Gate_C08`、`castletown00`）。只有名字和摆放位置，几何在被配对的 `.MBS` 里，所以只列出、不绘制。 | Layer field `+0x3e` indexes 337 prop names (`Town_C18_1`, `Gate_C08`, `castletown00`). It carries a name and a placement only; the geometry lives in the paired `.MBS`, so the viewer lists it but does not draw it. |
| **面板跨度 / Panel span** | 面板四个角点的包围跨度，是**纹素的 2 倍**（页 2048 → 跨度 4096）。世界尺寸 = `跨度 × 0.5 × 缩放`，即 1 纹素 = 1 世界单位。面板自身的 `x/y` 只是图集排版位置。 | The corner-to-corner span of an MPD panel, **twice the texel count** (a 2048 page gives 4096). World size is `span × 0.5 × scale`, so one texel is one world unit. The panel's own `x/y` is only its place in the atlas. |
| **视差 / Parallax** | 图层旗标 `+0x34` 的 bit6/bit7 会按 `摄像机字段 × 1e-4` 错动，只影响动态镜头；静态窗口可以不管。整族叠加原本以为不允许，其实是 `01P` 的 program 1 画的是带 alpha 的软云，盖住地面属于正常遮挡。见 ADR-0029。 | Flags bit6/bit7 of `+0x34` shift a layer by `camera field × 1e-4`, which only matters for a moving camera. Stacking a whole family was thought to be invalid; it is not — the `01P` kind (program 1) draws soft clouds with alpha, and covering the ground is correct occlusion. See ADR-0029. |
| **滚转 / Roll** | 图层 `+0x2e`，平面内旋转，单位 65536 = 一圈。`−32768`（180°）是最常见的一档。 | Layer field `+0x2e`, an in-plane rotation with 65536 units per turn. `−32768` (180°) is the common one. |
| **俯仰 / Pitch** | 图层 `+0x2c`，绕水平轴倾斜，只压纵向（`cos(pitch)`），不影响横向。455 层非零，多是 75° 的贴地板。 | Layer field `+0x2c`, a tilt about the horizontal axis: it only shortens the vertical extent by `cos(pitch)`. 455 layers use it, mostly 75° ground plates. |
| **世界窗口 / World window** | 从一个放置集的全景带上截取的一段世界 x 区间。看图就是移动这个窗口。 | A world-x interval cut out of a placement set's panorama. Panning means moving this window. |
| **模型层 / Model layer** | 摆放记录里引用 `.MBS` 的图层。它没有代理四边形，几何在被引用的模型里，而摆放依据尚未闭合。 | A layer referencing a `.MBS`. It carries no proxy quad; the geometry lives in the model, and the placement rule is not closed yet. |

## 动画格式层 / Animation format layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **MBS** | Vanillaware 动画族文件（MBP/MBS 同族）。二进制、按「分节」组织，非顺序结构。 | Vanillaware animation family file. Binary, organized into *sections*, not a sequential structure. |
| **分节表 / Section table** | 描述每个分节「从哪里读、读多少、步长多少」的数据驱动布局表。逐平台不同。 | Data-driven layout table describing each section's read offset, count and stride. Per-platform. |
| **sp** | 分节表给出的该分节数据的**绝对起始偏移**。 | Absolute start offset of a section's data, given by the table. |
| **sk / 步长** | 该分节每条记录的字节长度。 | Byte stride of one record in that section. |
| **s0..sb** | 分节编号，共 12 个。含义随平台（tag）变化，是格式的核心不变量。 | Section ids s0..sb (12 total). Meaning varies by platform tag — a core invariant of the format. |
| **tag** | 平台标识串，如 `swi_unic` / `ps4_unic` / `ps4_odin`。决定分节表、blend 表、mix 表。 | Platform identifier string (e.g. `swi_unic`). Selects the section table, blend table and mix table. |
| **V55** | 分节数据被摊平成 JSON 后的中间模型（`.mbs.v55`）。是「机器可读的 MBS」而非另一种格式。 | The flattened-JSON intermediate model of MBS sections. A machine-readable view of MBS, not a separate format. |
| **Blend 表** | 平台级的混合模式常量表（源/目标因子组合）。按 tag 硬编码。 | Platform-level constant table of blend modes (src/dst factor combos). Hardcoded per tag. |
| **Mix 表** | 平台级的插值方式表（LINEAR / CATMULL ROM 等）。按 tag 硬编码。 | Platform-level interpolation table (LINEAR / CATMULL ROM, …). Hardcoded per tag. |

## 四边形模型层 / Quad model layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **Quad** | 本工具的输出模型，同时也是内部渲染模型。对应 `.quad.json`。 | The tool's output model and internal render model. Serialized as `.quad.json`. |
| **dstquad** | 绘制目标四边形：屏幕/世界坐标下的四个顶点。 | Destination quad: the four corners in screen/world space. |
| **srcquad** | 纹理采样四边形：图集内的四个 UV 角点。 | Source quad: the four UV corners inside the atlas. |
| **fogquad** | 可选的雾色四边形数据。 | Optional fog-colour quad data. |
| **tex_id** | 纹理索引，指向该动画所用的图集。 | Texture index pointing at the atlas used by the animation. |
| **blend_id** | 该 keyframe 记录引用的 blend 表条目下标。 | Index into the blend table referenced by this keyframe entry. |
| **keyframe** | 一个动画帧：含一组 layer（每 layer 一个 dstquad + 可选 tex）。 | One animation frame: a set of layers, each with a dstquad and optional texture. |
| **hitbox** | 判定框，来自 s3 的 rect。 | Hit box, from section s3 rects. |
| **slot** | 骨骼/插槽条目，承载矩阵与时序。 | Skeleton/slot entry carrying a matrix and its timeline. |
| **skeleton** | 由多个 slot 组成的骨架。 | A skeleton composed of multiple slots. |
| **animation** | 一段具名时序，由 keyframe/hitbox 序列构成。 | A named timeline made of keyframe/hitbox sequences. |
| **pixlines** | 把若干矩形摊到一张图上的内部工具对象，用于生成 `.clut` 图集与调试图。 | Internal helper that packs rects into one image; used to build `.clut` atlases and debug images. |
| **图集 / Atlas** | 由 mbs 内嵌纹理按 `tex_id` 摊平成的贴图，命名 `<quad名>.<tex_id>.png`。运行期由 `.mbs` 同目录的纹理容器现解（ADR-0007）。 | Texture sheet flattened from mbs-embedded textures by `tex_id`, named `<quad>.<tex_id>.png`. Decoded at runtime from the texture container next to the `.mbs` (ADR-0007). |
| **纹理容器归属 / Container resolution** | `.mbs` 找纹理容器的规则：先找同名 `.ftx`/`.ftp`，否则找「同名 + 纯数字后缀」的**唯一**候选；候选多于一个则报错不猜（如 `Eltlinde_F` 的 `00`/`10`）。 | How an `.mbs` resolves its texture container: exact stem first, then the unique `<stem><digits>` variant; more than one candidate is an error, never a guess. |
| **预览场景 / Preview scene** | 预览器用的一次性快照：图集帧元信息 + 已解好引用的 keyframe/hitbox 图层 + 各骨的时间线。前端不接触 V55。 | The one-shot snapshot consumed by the previewer: atlas frame metadata, reference-resolved keyframe/hitbox layers, and per-bone timelines. The front end never sees V55. |
| **预览渲染器 / Preview renderer** | PixiJS 8 的场景图：相机 Container + 图层 Mesh 池 + 调试 Graphics，播放由 ticker 推进。见 ADR-0011。 | The PixiJS 8 scene graph: camera container, pooled layer meshes and debug graphics, with playback advanced by the ticker. See ADR-0011. |

## 契约层 / Contract

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **真值 / Golden** | 由 PHP 权威工具跑出的产物，用作逐字段比对基准。 | Artifacts produced by the authoritative PHP tool, used as byte/field comparison baselines. |
| **结构等价 / Structural equivalence** | JSON 校验判据：递归比较键值，忽略键序与空白，容忍缺省键差异。见 ADR-0006。 | The JSON verification predicate: recursive key/value comparison ignoring key order and whitespace. |
| **自洽性检查 / Consistency check** | 无 golden 平台（v76/v72）的校验强度：解析不崩 + 索引/尺寸/引用不变量成立。见 ADR-0006。 | Verification strength for platforms without golden: parse must not fail, and index/size/reference invariants must hold. |
| **文档树 / Document Tree** | 把任意格式投影成同一套扁平 arena 节点；前端按节点 id 分页展开，不一次性传输整棵树。见 ADR-0018。 | A flat arena projection of any format; the front end expands nodes by id in pages instead of receiving the whole tree at once. See ADR-0018. |
| **文档句柄 / Doc Handle** | 文档缓存的 `u64` 标识；缓存淘汰后失效，调用方必须重新打开。 | A `u64` identifier for a cached document; invalid after eviction, so callers must reopen. |
| **游戏档案 / Game Profile** | 由目录名嗅探得到的游戏身份，只决定默认 mbs tag 与优先查看器，不参与可见性。 | Game identity inferred from directory names; only supplies a default mbs tag and preferred viewers, never visibility. |
| **反汇编 / Disassembly** | ASB 字节码 → 助记符 + 操作数 + 跨表注记（调用目标、全局、跳转标签）。 | ASB bytecode → mnemonics plus operands and cross-table notes (call targets, globals, jump labels). |
| **伪代码 / Pseudo-code** | 对表达式栈做模拟得到的可读语句；控制流保留 label + goto，不追求可编译。 | Readable statements from simulating the expression stack; control flow stays as label + goto, not compilable. |
| **布局示意图 / Layout schematic** | BBS 的 bbox、物件、雾区四边形与相机轨迹的 SVG 示意，不是真实渲染。 | An SVG schematic of a BBS bbox, objects, fog quads and camera routes — not a real render. |
| **图元层 / Primitive Layer** | 与格式无关的可视化 DTO：场景 → 节点 → 网格/引用；FMD/OSB/RSC 各自投影进来。见 ADR-0019。 | Format-agnostic visualization DTOs: scene → nodes → mesh/reference; FMD, OSB and RSC each project into it. See ADR-0019. |
| **资源引用 / Resource Reference** | 场景节点指向另一个资源文件的关系（模型/纹理/动画/音频/脚本），带 `role` 与 `resolved/missing` 状态。 | A scene node pointing at another resource file (model/texture/animation/audio/script), with a role and resolved/missing status. |
| **网格范围 / Sub Range** | FMD 一个绘制段的顶点区间与三角数；首版用它生成顺序索引，按段着色/线框。 | A draw segment's vertex range and triangle count in FMD; the first slice derives sequential indices and colors/wires by range. |

## 界面层 / UI layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **主题 / Theme** | daisyUI 内置配色方案，经 `<html data-theme>` 切换；默认 dark，选择存于 `localStorage["vw-theme"]`。见 ADR-0017。 | A daisyUI built-in color scheme switched through `<html data-theme>`; dark by default, persisted in `localStorage["vw-theme"]`. See ADR-0017. |
| **语义色 / Semantic Color** | 随主题变化的 daisyUI 颜色名（`base-*`、`primary`、`success` 等）；界面不得出现固定色值。 | Theme-aware daisyUI color names (`base-*`, `primary`, `success`, …). Fixed color values must not appear in the UI. |
| **列 / Column** | 分栏视图里的一列：对应一个目录，列头是它的名字，列体是它的目录项与文件。第一列永远是根。见 ADR-0026。 | One ledger of the column view: one directory, its name in the header, its entries below. The first column is always the root. See ADR-0026. |
| **预览栏 / Preview pane** | 右侧固定宽度的面板：选中文件时给出元信息与查看器 tab，选中目录时给出目录或归档概览。 | The fixed-width panel on the right: metadata plus viewer tabs for a file, a folder or archive overview for a directory. |
| **钉住 / Pin** | 双击或回车把当前文件固定到预览栏并装载查看器；换选即取消钉住。 | Double-click or Enter fixes the file into the preview pane and loads its viewer; changing the selection unpins. |
