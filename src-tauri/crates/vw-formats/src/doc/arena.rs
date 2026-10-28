//! 文档树 arena：节点以数组下标为 id，子节点只存 id，分页按需切片。
//!
//! 这里只有 DTO 与容器，不含任何格式知识；各格式解析器通过 [`Builder`] 建树。

use serde::Serialize;

pub const PAGE_LIMIT: u32 = 100;
pub const PAGE_MAX: u32 = 500;
pub const MAX_NODES: usize = 100_000;
pub const MAX_DEPTH: usize = 64;
pub const MAX_STRING: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocNodeKind {
    Root,
    Struct,
    Array,
    Field,
    Bytes,
    Text,
    Opaque,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DocMark {
    Note,
    Warn,
    Error,
}

/// 扁平节点。`children` 只存 id，不下发到前端。
#[derive(Debug, Clone)]
pub struct DocNode {
    pub id: u32,
    pub parent: Option<u32>,
    pub key: Option<String>,
    pub name: String,
    pub kind: DocNodeKind,
    pub value: Option<String>,
    pub offset: Option<u64>,
    pub len: Option<u64>,
    pub mark: Option<DocMark>,
    pub children: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocNodeDto {
    pub id: u32,
    pub parent: Option<u32>,
    pub key: Option<String>,
    pub name: String,
    pub kind: DocNodeKind,
    pub value: Option<String>,
    pub offset: Option<u64>,
    pub len: Option<u64>,
    pub mark: Option<DocMark>,
    pub child_count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocCheckDto {
    pub id: String,
    pub label: String,
    pub ok: bool,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocDto {
    pub handle: u64,
    pub path: String,
    pub ext: String,
    pub label: String,
    pub size: u64,
    pub total_nodes: u32,
    pub build_ms: u32,
    pub complete: bool,
    pub truncated: bool,
    pub checks: Vec<DocCheckDto>,
    pub warnings: Vec<String>,
    pub root: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocPageDto {
    pub handle: u64,
    pub node: u32,
    pub offset: u32,
    pub total: u32,
    pub items: Vec<DocNodeDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DocOpenDto {
    pub doc: DocDto,
    pub page: DocPageDto,
}

/// 文档树容器。
#[derive(Debug, Clone)]
pub struct DocArena {
    nodes: Vec<DocNode>,
    checks: Vec<DocCheckDto>,
    warnings: Vec<String>,
    truncated: bool,
}

impl DocArena {
    pub fn root(&self) -> u32 {
        0
    }

    pub fn total_nodes(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// 对所有 key 匹配的字段值求和（语料测试用）。
    pub fn sum_field(&self, key: &str) -> u64 {
        self.nodes
            .iter()
            .filter(|node| node.key.as_deref() == Some(key))
            .filter_map(|node| node.value.as_deref())
            .filter_map(|value| value.parse::<u64>().ok())
            .sum()
    }

    /// 统计 key/value 完全匹配的节点数（语料测试用）。
    pub fn count_field_value(&self, key: &str, value: &str) -> usize {
        self.nodes
            .iter()
            .filter(|node| {
                node.key.as_deref() == Some(key) && node.value.as_deref() == Some(value)
            })
            .count()
    }

    pub fn approx_bytes(&self) -> usize {
        let payload: usize = self
            .nodes
            .iter()
            .map(|node| {
                node.name.len()
                    + node.key.as_deref().map_or(0, str::len)
                    + node.value.as_deref().map_or(0, str::len)
            })
            .sum();
        payload + self.nodes.len() * std::mem::size_of::<DocNode>()
    }

    /// 取某节点的子节点页；节点不存在时返回 `None`。
    pub fn page(&self, handle: u64, node: u32, offset: u32, limit: u32) -> Option<DocPageDto> {
        let target = self.nodes.get(node as usize)?;
        let limit = limit.clamp(1, PAGE_MAX);
        let offset = offset as usize;
        let total = target.children.len() as u32;

        let items = target
            .children
            .iter()
            .skip(offset)
            .take(limit as usize)
            .filter_map(|id| self.nodes.get(*id as usize))
            .map(|child| DocNodeDto {
                id: child.id,
                parent: child.parent,
                key: child.key.clone(),
                name: child.name.clone(),
                kind: child.kind,
                value: child.value.clone(),
                offset: child.offset,
                len: child.len,
                mark: child.mark,
                child_count: child.children.len() as u32,
            })
            .collect();

        Some(DocPageDto {
            handle,
            node,
            offset: offset as u32,
            total,
            items,
        })
    }

    /// 自检结果（测试与调试用；对外通过 [`Self::doc`] 返回）。
    pub fn checks(&self) -> &[DocCheckDto] {
        &self.checks
    }

    pub fn doc(
        &self,
        handle: u64,
        path: String,
        ext: String,
        label: String,
        size: u64,
        build_ms: u32,
    ) -> DocDto {        DocDto {
            handle,
            path,
            ext,
            label,
            size,
            total_nodes: self.total_nodes(),
            build_ms,
            complete: !self.truncated,
            truncated: self.truncated,
            checks: self.checks.clone(),
            warnings: self.warnings.clone(),
            root: self.root(),
        }
    }
}

/// 建树器：所有方法都做节点上限保护，越界时只置 `truncated`。
#[derive(Debug)]
pub struct Builder {
    nodes: Vec<DocNode>,
    checks: Vec<DocCheckDto>,
    warnings: Vec<String>,
    truncated: bool,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            checks: Vec::new(),
            warnings: Vec::new(),
            truncated: false,
        }
    }

    pub fn root(&mut self, name: impl Into<String>) -> u32 {
        self.push(None, None, name, DocNodeKind::Root, None, None, None)
    }

    pub fn group(&mut self, parent: u32, key: Option<&str>, name: impl Into<String>) -> u32 {
        self.push(Some(parent), key, name, DocNodeKind::Struct, None, None, None)
    }

    pub fn array(&mut self, parent: u32, key: Option<&str>, name: impl Into<String>) -> u32 {
        self.push(Some(parent), key, name, DocNodeKind::Array, None, None, None)
    }

    pub fn field(
        &mut self,
        parent: u32,
        key: &str,
        value: impl Into<String>,
    ) -> u32 {
        self.push(Some(parent), Some(key), key, DocNodeKind::Field, Some(value.into()), None, None)
    }

    pub fn bytes(
        &mut self,
        parent: u32,
        key: Option<&str>,
        name: impl Into<String>,
        offset: u64,
        len: u64,
    ) -> u32 {
        self.push(Some(parent), key, name, DocNodeKind::Bytes, None, Some(offset), Some(len))
    }

    pub fn text(
        &mut self,
        parent: u32,
        key: &str,
        name: impl Into<String>,
        value: impl Into<String>,
        offset: Option<u64>,
    ) -> u32 {
        self.push(Some(parent), Some(key), name, DocNodeKind::Text, Some(value.into()), offset, None)
    }

    pub fn opaque(&mut self, parent: u32, key: &str, name: impl Into<String>) -> u32 {
        self.push(Some(parent), Some(key), name, DocNodeKind::Opaque, None, None, None)
    }

    pub fn mark(&mut self, id: u32, mark: DocMark) -> &mut Self {
        if let Some(node) = self.nodes.get_mut(id as usize) {
            node.mark = Some(mark);
        }
        self
    }

    pub fn check(&mut self, id: &str, label: impl Into<String>, ok: bool, detail: Option<String>) {
        self.checks.push(DocCheckDto {
            id: id.to_string(),
            label: label.into(),
            ok,
            detail,
        });
    }

    pub fn warn(&mut self, message: impl Into<String>) {
        if self.warnings.len() < 32 {
            self.warnings.push(message.into());
        }
    }

    pub fn truncate(&mut self) {
        self.truncated = true;
    }

    pub fn finish(self) -> DocArena {
        DocArena {
            nodes: self.nodes,
            checks: self.checks,
            warnings: self.warnings,
            truncated: self.truncated,
        }
    }

    // 建树入口参数虽多，但都是一次性写入的节点字段；拆成 options struct 反而更难读。
    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: Option<u32>,
        key: Option<&str>,
        name: impl Into<String>,
        kind: DocNodeKind,
        value: Option<String>,
        offset: Option<u64>,
        len: Option<u64>,
    ) -> u32 {
        if self.nodes.len() >= MAX_NODES {
            self.truncated = true;
            return parent.unwrap_or(0);
        }

        let id = self.nodes.len() as u32;
        let mut name = name.into();
        if clamp_string(&mut name) {
            self.truncated = true;
        }
        let value = value.map(|mut text| {
            if clamp_string(&mut text) {
                self.truncated = true;
            }
            text
        });

        self.nodes.push(DocNode {
            id,
            parent,
            key: key.map(str::to_string),
            name,
            kind,
            value,
            offset,
            len,
            mark: None,
            children: Vec::new(),
        });

        if let Some(parent) = parent {
            if let Some(node) = self.nodes.get_mut(parent as usize) {
                node.children.push(id);
            }
        }

        id
    }
}

/// 按 UTF-8 字符边界截断；返回是否发生截断。
fn clamp_string(text: &mut String) -> bool {
    if text.len() <= MAX_STRING {
        return false;
    }
    let mut end = MAX_STRING;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push('…');
    true
}
