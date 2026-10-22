# CONTEXT — Vanillaware 工具链领域词汇表

本文件是本仓库的**唯一术语权威**（glossary）。任何 issue、ADR、测试名、重构提案在命名领域概念时必须使用此处定义的词，不得另造同义语。

This file is the repository's authoritative glossary. When naming a domain concept in issues, ADRs, test names, or refactor proposals, use the term defined here. Do not invent synonyms.

> 维护方式：术语在决策真正落地时才追加，不预先臆造。中英并列，中文为准。
> Maintenance: terms are appended lazily when a decision lands. Chinese is normative.

---

## 资产层 / Asset layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **文件条目 / File Entry** | 库里的最小展示单位：磁盘上的一个真实文件。以绝对路径为键。 | The smallest unit shown in the library: one real file on disk, keyed by absolute path. |
| **目录列表 / Directory Listing** | 某个目录的即时读取结果，含子目录与文件条目。列表以此为准，索引不能替代它。 | The live read of one directory: child dirs plus file entries. Listings are authoritative; the index never replaces them. |
| **格式描述符 / Format Descriptor** | 一种文件格式的能力声明：类别、图标、可用查看器、有无缩略图。后端是唯一真值。 | Capability declaration for a file format: kind, icon, available viewers, thumbnail support. Backend is the single source of truth. |
| **类别 / Kind** | 格式的粗分类，**只含原生游戏二进制资源**：纹理、动画、场景、模型、脚本、文本、列表、网格、二进制。图片与文本文件不属于本工具的收录范围；扩展名级差异由「格式」声明。 | Coarse class of **native game binary resources only**: texture, animation, scene, model, script, text, list, grid, binary. Image and text files are out of scope; extension-level differences live in Format. |
| **格式 / Format** | 扩展名级的能力声明：`ext / kind / label / viewers / has_thumb`。查看器与缩略图能力跟随格式，不跟随类别。见 ADR-0018。 | Extension-level capability declaration: `ext / kind / label / viewers / has_thumb`. Viewer and thumbnail capabilities follow the format, not the kind. See ADR-0018. |
| **查看器 / Viewer** | 针对某格式打开的呈现组件。同一文件可挂多个查看器（如动画的「播放」「结构」与「十六进制」）。 | A presentation component opened for a format. One file may have several viewers (e.g. animation playback, structure, hex). |
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
| **二进制类别 / Binary kind** | 已确认是原生资源、解析器尚未进场的族（如 `.nnd`、`.bbs`）。可看元信息与头部字节，不做解读。 | A native resource whose parser has not landed yet. Metadata and leading bytes only. |
| **收录 / Catalogued** | 只有格式注册表里登记的原生资源才被收录。`.png`、`.json`、`.txt`、`.quad` 等派生产物在目录列表里被过滤掉，只报数量。 | Only registry-listed native resources are catalogued. Derived artifacts (`.png`, `.json`, `.txt`, `.quad`) are filtered out of listings; only a count is reported. |

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

## 界面层 / UI layer

| 术语 | 中文释义 | English |
| --- | --- | --- |
| **主题 / Theme** | daisyUI 内置配色方案，经 `<html data-theme>` 切换；默认 dark，选择存于 `localStorage["vw-theme"]`。见 ADR-0017。 | A daisyUI built-in color scheme switched through `<html data-theme>`; dark by default, persisted in `localStorage["vw-theme"]`. See ADR-0017. |
| **语义色 / Semantic Color** | 随主题变化的 daisyUI 颜色名（`base-*`、`primary`、`success` 等）；界面不得出现固定色值。 | Theme-aware daisyUI color names (`base-*`, `primary`, `success`, …). Fixed color values must not appear in the UI. |
