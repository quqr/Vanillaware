# ADR-0006：校验判据为 JSON 结构等价；分平台强度不同

- **状态**：已接受（2026-10-20）
- **影响范围**：测试、转换视图的 diff、CI

## 背景

PHP 产出的 `.quad.json` 是 pretty-printed，键序由 PHP `array` 插入序决定，且会**缺省省略**空字段
（例如 `fogquad` 仅在 `s0v !== '#ffffffff'` 时写入；`attribute` / `colorize` 同理）。
要求 Rust 逐字节复现等于同时复现 PHP 的键序与省略规则——那不是格式的一部分，是实现的偶然。

同时，语料只覆盖 v77：`Main-Output/quads` 的 722 个 `.quad.json` 全是 Unicorn Overlord。
v76/v72 家族**没有** golden。

## 决策

- **判据 = 结构等价**：递归比较键值；忽略键序与空白；把「缺失键」与「显式为 null/空」视作等价，
  但**值不等即失败**，并输出字段级路径（`skeleton[3].slot.name`）便于定位。
- **v77**：对全部 722 个 golden 做逐字段结构等价；任何差异即失败。
- **v76/v72**：无 golden，只做**自洽性检查**——解析不崩、不 panic，且以下不变量成立：
  索引不越界、`srcquad` 落在图集范围内、矩阵可分解、引用目标存在、计数与记录数一致。
  这些检查是**如实的能力边界声明**，不得伪造「通过」。
- 校验器复用同一份代码，仅切换「比对模式」与「检查模式」。

## 修订（2026-10-20，第二次）

写作本 ADR 时假设「golden `.quad.json` 由 PHP 产出」。该假设**已被证伪**：
`F:\Codes\UNICORN OVERLORD\extract\Main-Output\quads` 的 722 个 golden 是旧 C# 工具
（`f:\Codes\QuadToSpine2D\VanillawareConverter\MBSConverter\Converters\V55ToQuadConverter.cs`）的产物，
与 PHP 参考存在**算法级**差异，不是同一实现的两个序列化形态。

已确认的已知差异（穷举，不得再增不减）：

| # | 差异点 | C#（golden） | PHP（真值） |
| --- | --- | --- | --- |
| 1 | s7 矩阵平移合成 | `m = m * Translate(..)`（右乘） | `m[3] += move[0]*bx; m[7] += move[1]*by; m[11] += move[2]`（平移列直接相加） |
| 2 | blend 条目结构 | `[{"debug":"normal"}]`（仅 debug 键） | `quadfile::blend('normal')` 完整结构 |
| 3 | mix 首项 | `""` | `0` |
| 4 | 空位填充值 | `null`（层数组空位） | 不输出该位（`list_add` 补 `0`） |
| 5 | `attribute` | `int` | `bitflag_attr(...)` 字符串数组 |
| 6 | `fogquad` 出现条件 | 无条件输出 | 仅 `s0v !== '#ffffffff'` |
| 7 | 顶层键 | `blend/mix/keyframe/hitbox/slot/animation/skeleton`（**无 `tag`**） | 多一个顶层 `tag`（idtag 解码结果） |
| 8 | `matrix` 键与单位矩阵 | 键恒存在，单位矩阵写**完整 16 个 f64** | 键恒存在（PHP 的 `$matrix !== 0` 对数组恒真），单位矩阵写 `[]` |
| 9 | 浮点来源 | f32 原始值直接算矩阵 | 先经 `jsonpretty::number_format($v, 6)` 收敛成 6 位小数再算 |
| 10 | `color` 缺省 | 写 `null` | 等于 `#ffffffff` 时**整键省略** |
| 11 | 浮点落盘精度 | `float` 的**最短往返**表示（约 7 位有效数字，如 `2218.0312`） | f32 值精确展宽成 f64（`2218.03125`） |
| 12 | s0 雾色的读法 | `ParseNdsQuad18c` **只读首色**（`0x04` 的 4 字节） | `fog_quad` 读 `0x04/0x08/0x0c/0x10` 四色，全等才退化成单串 |
| 13 | `layer` 尾部空位 | 只在真正写入某条时才 `Add(null)`，尾部空位被丢掉 | `list_add` 先按条数补满 `0`，尾部空位保留 |
| 14 | `attribute` 的数值上限 | `int.TryParse(hex, HexNumber)`：`>= 0x80000000` 溢出解析失败，按键 `0` 处理并省略整键 | `hexdec` 无上限，照常输出 `keyattr_80000000` |

因此**修订决策**：

- **Rust 输出以 PHP 为准**（ADR-0004 不变）：Quad 采用 PHP 原样 schema——`blend` 写完整
  `{name,mode_rgb,mode_alpha,debug}`、`mix` 首项为 `0`、省略空字段、空位 padding 写 `0`。
- **v77 校验强度 = 结构等价 + 可解释差异清单**：上表 14 项差异命中时**不判失败**，
  记入差异报告；**其余字段任何差异判失败**。清单是**闭集**，新增差异必须回到本 ADR 修订。
- 浮点比较用容差 `1e-6`（PHP 经 `number_format(x,6)`），并按第 11 项加 f32 有效位的相对项。
- **golden 比对前先把 C# 一侧规范化到 PHP 形态**：按上表补全 `blend`、`mix[0]` 由 `""` 改 `0`、
  `null` 空位改 `0`、补出被省略的键、`attribute` 展开成 `bitflag_attr` 字符串数组、
  补出顶层 `tag`（从 `.mbs` 的 `id3` 侧解码，与 Rust 侧同源，故不构成独立验证）。
- 第 1 项**无法规范化**（两者是不同算法）：矩阵比对只覆盖 3×3 块（下标 `0,1,2` / `4,5,6` / `8,9,10`），
  平移列（`3,7,11`）的任何差异记入报告但不判失败——这是**如实登记的能力边界**，
  不是「已验证」。第 9 项同理只影响 3×3 块内的数值，落在容差内。
- 第 11 项：数值比较用「绝对 `1e-6` + 相对 `2e-7`」，后者小于 1 个 f32 ulp（`2^-23 ≈ 1.19e-7`
  的整 ulp），故只能吸收落盘精度差，不能掩盖真正的错位。相对项对所有数值生效，因为比较器
  无法分辨哪些 f64 来自 f32。
- 第 12 项：比对前把**两侧**的 `fogquad` 投影成「首色」（四色数组取 `[0]`，等于 `#ffffffff`
  时删除该键）。因此 golden 只验证首色与「首色为 `#ffffffff` 时省略」这条规则；
  **第 2~4 个色在本 golden 下不可验证**，属能力边界。
- 第 13 项：比对前把**两侧**的 `keyframe[*].layer` / `hitbox[*].layer` 去掉尾部空位；
  尾部空位不承载信息，中间空位两边一致，仍逐位比较。
- 第 14 项：任一侧的 `attribute` 出现 `keyattr_80000000` 时，两侧一律删键；其余位仍逐项比较。
- 矩阵的 3×3 块容差按**整块最大分量**取 `1e-5 + 1e-5 * max|v|`：误差随缩放线性放大，
  按单元素取相对项会把「大缩放 × 近零元素」误判（实测有 2 帧命中）。
- v76/v72 仍只有自洽性检查（无 golden）。
- **日常测试默认分层抽样**：按 `.mbs` 所在目录分组（Chara / BattleBG / GUI / …），组内用固定种子的
  哈希排序后按比例取样、每组至少 2 个，共约 32~40 个；保留 `VW_MBS_FULL_VERIFY=1` 跑全量。
  抽样只决定「一次 `cargo test` 读多少 golden」，不改变判据；发版前必须跑一次全量。

## 权衡

代价：结构等价可能漏掉「键序」这类差异，但那正是我们**故意**不想比较的；若将来需要与外部工具做逐字节比对，
必须另加一个独立的严格模式，不能把这个判据悄悄收紧。

收益：判据与「格式正确性」对齐，不被序列化细节绑架；v76/v72 的不足被显式登记，而不是隐藏在绿灯里。

## 备选方案

- **逐字节一致**：最严，但会被 pretty-print、键序、省略规则卡死，且这些不是格式语义。
- **关键字段抽样**：松，容易漏掉整类 bug（例如整个 s7 错位）。

---

## English summary

Golden `.quad.json` is pretty-printed with PHP-array key order and omits empty fields, so byte-identity is
not a format property. The predicate is **structural equivalence**: recursive key/value comparison, ignoring
key order and whitespace, with field-level diff paths; any value mismatch fails. v77 is checked field-by-field
against all 722 goldens. v76/v72 have no golden, so they get **consistency checks only** (no panic plus index,
bounds, reference and count invariants) — an honest statement of capability, never a faked pass.

Revision 2 (2026-10-20): the 722 goldens were produced by the old C# tool, not by PHP, so the two differ at
the **algorithm level**. There is a closed set of 10 accepted differences (matrix translation composition,
blend entry shape, `mix[0]`, `null` vs `0` padding, `attribute` type, `fogquad` condition, the extra top-level
`tag`, `matrix: []` for identity, the 6-decimal float rounding the PHP converter feeds on, and `color: null`
vs a missing key). Rust output follows PHP (ADR-0004). Before diffing, the C# side is **normalised** to the PHP
shape; items 1 and 9 cannot be normalised, so matrix comparison covers only the 3×3 block — translation column
differences are reported but do not fail. That gap is a declared capability boundary, not a verified result.

Revision 3 (2026-10-21), after the first full-corpus run, adds three rows (11–13), all discovered empirically:

- **11 — float print precision.** C# writes `float` in shortest round-trip form (`2218.0312`), we widen the f32
  exactly (`2218.03125`). Numeric comparison therefore allows `1e-6` absolute **plus** `2e-7` relative, which is
  less than one f32 ulp and cannot mask real misalignment.
- **12 — s0 fog read.** `ParseNdsQuad18c` reads only the **first** colour at `0x04`; PHP's `fog_quad` reads four
  colours at `0x04/0x08/0x0c/0x10` and degrades to one string only when all four match. Before diffing, both
  sides' `fogquad` is **projected to the first colour** (a four-colour array collapses to `[0]`, and is removed
  when that colour is `#ffffffff`). Colours 2–4 are therefore **not verifiable** against this golden.
- **13 — trailing layer pads.** PHP's `list_add` pre-pads the whole layer with `0`, the C# tool only extends the
  list when it actually writes an entry, so trailing pads are missing there. Both sides are trimmed of trailing
  pads before diffing; interior pads still compare element by element.
- **14 — attribute numeric range.** C# parses `attr` with `int.TryParse(hex, HexNumber)`; values `>= 0x80000000`
  overflow, parse as 0, and the whole `attribute` key is omitted. PHP's `hexdec` has no such limit. When either
  side carries `keyattr_80000000`, both keys are dropped; the remaining bits still compare.

Matrix comparison uses `1e-5 + 1e-5 * max|block element|`, because the error caused by PHP's 6-decimal input
rounding scales with the matrix magnitude (two frames in the corpus exceed a per-element relative bound).

Corpus note: the 722 goldens are produced from the `extract\000` tree. Three stems (`Interface`,
`Interface_Button`, `Opening00`) also exist under `art book ex` with different content, so the validator
**refuses to guess** and reports them as skipped; pointing `VW_MBS_MBS_ROOT` at `extract\000` covers all 722.
