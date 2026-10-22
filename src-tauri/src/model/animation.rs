//! 动画转换结果的 DTO。

use serde::Serialize;

/// 单文件 `V55 → Quad` 的结果与自检。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertDto {
    pub mbs_path: String,
    pub tag: String,
    /// 转出来的 Quad（PHP 原样 schema），前端直接渲染成树。
    pub quad: serde_json::Value,
    /// 数组长度、`name`、索引越界等结构不变量。
    pub self_check: Vec<String>,
}
