//! V55 中间模型：MBS 分节被摊平成 JSON 后的**机器可读视图**（见 `CONTEXT.md`）。
//!
//! 本文件是 PHP `FMBP_FMBS_vanillaware.php` 输出对象的**逐字段誊写**：
//! 键名、键序、取值形态（`bits` / `attr` 是 `'0x…'` 串、`fog` 是 `'#…'` 串、
//! `s0s1s2` / `s4` / `s5` / `s8` / `sa` 是定长整形数组）都与 PHP 一致。
//! 之所以不「优化」成更自然的结构，是因为 `.mbs.v55` 要与 PHP 产物做结构等价比对，
//! 任何再设计都会把序列化细节混进判据（ADR-0004：真值是 PHP）。
//!
//! 顶层 `ver` 恒为 `"55"`：这是 PHP 的**格式版本号常量**，与文件头里的 `0x72/0x76/0x77`
//! 是两个概念（后者是布局代际，见 [`super::profile::Profile::version`]）。

use serde::{Deserialize, Serialize};

/// s0 记录：一组雾色（PHP `nds_quad18c` → `fog_quad(s, 0x04, 0x08, 0x0c, 0x10)`）。
///
/// 注意：PHP 的 s0 / s1 / s2 记录**没有** `i` 包裹，直接就是取值本身。
/// 后 9 个分节才有 `{ "i": "s3 0", … }` 对象。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FogQuad {
    /// 四色全等 → `"#rrggbbaa"`。
    One(String),
    /// 四色不全等 → `["#…", "#…", "#…", "#…"]`。
    Four([String; 4]),
}

/// s3 记录：判定框矩形 + 12 个附加 float。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S3Rec {
    pub i: String,
    /// 8 个 float：`x1 y1 x2 y2 x3 y3 x4 y4`。
    pub rect: Vec<f32>,
    /// 12 个 float（PHP 原样输出，未在转换器里使用）。
    pub xyz: Vec<f32>,
}

/// s4 记录：一个绘图层（blend / tex / 三角索引 / flags / attrib / color）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S4Rec {
    pub i: String,
    /// blend 表下标。
    pub blend: i64,
    /// 图集（tex_id）下标。
    pub tex: i64,
    /// `[s0, s1, s2]` 三个分节的记录下标（PHP 顺序固定如此）。
    pub s0s1s2: Vec<i64>,
    /// flags 的 `'0x…'` 十六进制串。
    pub bits: String,
    /// attrib 的 `'0x…'` 十六进制串。
    pub attr: String,
    /// 颜色（BRGA 打包值，原样保留）。
    pub color: i64,
}

/// s5 记录：s3 引用 + flags。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S5Rec {
    pub i: String,
    /// 引用的 s3 记录下标。
    pub s3: i64,
    /// flags 的 `'0x…'` 串。
    pub bits: String,
}

/// s6 记录：帧矩形 + s4 / s5 的索引区间。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S6Rec {
    pub i: String,
    /// `left top right bottom`。
    pub rect: Vec<f32>,
    /// `[起点, 条数]`。
    pub s4: Vec<i64>,
    /// `[起点, 条数]`。
    pub s5: Vec<i64>,
    /// flags 的 `'0x…'` 串。
    pub bits: String,
}

/// s7 记录：变换（平移 / 旋转 / 缩放）+ 雾色。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S7Rec {
    pub i: String,
    /// 3 个平移分量。
    #[serde(rename = "move")]
    pub move_: Vec<f32>,
    /// 3 个旋转分量。
    pub rotate: Vec<f32>,
    /// 2 个缩放分量。
    pub scale: Vec<f32>,
    /// `'#…'` 四字节雾色。
    pub fog: String,
}

/// s8 记录：时间线条目（帧数 / 回跳 / 各类占用标志）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S8Rec {
    pub i: String,
    /// 引用的 s6 记录下标。
    pub s6: i64,
    /// 引用的 s7 记录下标。
    pub s7: i64,
    /// 时长（帧）。
    pub time: i64,
    /// 回跳目标 s8 下标。
    #[serde(rename = "loop")]
    pub loop_: i64,
    /// 音效 id。
    pub sfx: i64,
    /// flags 的 `'0x…'` 串。
    pub bits: String,
    pub in_s5s3: i64,
    pub in_s7: i64,
    pub in_s6: i64,
    pub in_s0s1s2: i64,
}

/// s9 记录：具名矩形 + sa 索引区间。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct S9Rec {
    pub i: String,
    /// `left top right bottom`。
    pub rect: Vec<f32>,
    /// 0x18 字节的 ASCII 名（截断于首个 NUL）。
    pub name: String,
    /// `[起点, 条数]`。
    pub sa: Vec<i64>,
}

/// sa 记录：s8 的索引区间。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaRec {
    pub i: String,
    /// `[实际起点, 条数, 总帧数]`（起点已按 `st` 修正，见 PHP `FMBS_sa`）。
    pub s8: Vec<i64>,
}

/// sb 记录：次要区间；PHP 只输出标识，其余字段未使用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SbRec {
    pub i: String,
}

/// 一个 `.mbs` 摊平后的完整 V55 视图。
///
/// 键序与 PHP 的插入序一致：先 s0..sb，再 `tag` / `id3` / `ver`。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct V55 {
    /// s0：雾色，元素为 [`FogQuad`]。
    pub s0: Vec<FogQuad>,
    /// s1：源纹理四边形（每条 8 个 float）。
    pub s1: Vec<Vec<f32>>,
    /// s2：目标四边形（每条 8 个 float）。
    pub s2: Vec<Vec<f32>>,
    pub s3: Vec<S3Rec>,
    pub s4: Vec<S4Rec>,
    pub s5: Vec<S5Rec>,
    pub s6: Vec<S6Rec>,
    pub s7: Vec<S7Rec>,
    pub s8: Vec<S8Rec>,
    pub s9: Vec<S9Rec>,
    pub sa: Vec<SaRec>,
    pub sb: Vec<SbRec>,
    /// 平台标签（见 [`super::profile::Profile::tag`]）。
    pub tag: String,
    /// 平台 idtag（见 [`super::profile::Profile::id3`]）。
    pub id3: String,
    /// 格式版本号常量，恒为 `"55"`。
    pub ver: String,
}
