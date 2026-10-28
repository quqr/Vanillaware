//! SCB 图鉴 / 用语集文档：令牌流容器。
//!
//! 布局（18/18 文件实测零反例）：
//!
//! ```text
//! +0x00  u32 tokenCount
//! 之后   tokenCount × { u32 len ; u8[len] }   // len 含结尾 NUL，正文为 UTF-8
//! ```
//!
//! `tokenCount` 与条目数恒等，令牌流精确消费到 EOF。令牌本身是一棵
//! s-expression 文档树：
//!
//! ```text
//! ROOT { @MONSTER ( "Kurabe00" ) { Name ( "鞍部 十郎" ) ; PosY ( 0 ) ; } }
//! ```
//!
//! `@` 开头是子节点（可带一个标签括号），其余是属性（`名字 ( 值 ) ;`）；
//! 值可以是字符串、数字或逗号分隔的列表，实测嵌套深度不超过 1。

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

/// 令牌上限：单文件实测最多 4125，这里留两个数量级余量防呆。
const MAX_TOKENS: u32 = 200_000;
/// 单条令牌上限（含 NUL）。
const MAX_TOKEN_BYTES: usize = 0x4000;

struct Token {
    text: String,
    offset: u64,
}

pub fn build(bytes: &[u8]) -> DocArena {
    let mut builder = Builder::new();
    let root = builder.root("SCB");
    builder.field(root, "size", bytes.len().to_string());

    if bytes.len() < 4 {
        builder.warn("文件不足 4 字节");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let count = rdr::u32_at(bytes, 0).unwrap_or(0);
    builder.field(root, "tokenCount", count.to_string());

    let (tokens, end) = read_tokens(bytes, count, &mut builder);

    builder.check(
        "scb.eof",
        "令牌流精确到 EOF",
        tokens.len() == count.min(MAX_TOKENS) as usize && end == bytes.len(),
        Some(format!("{} 条 · 止于 0x{end:x} / 0x{:x}", tokens.len(), bytes.len())),
    );

    let doc = builder.group(root, Some("root"), "ROOT");
    parse_document(&tokens, doc, &mut builder);

    builder.finish()
}

fn read_tokens(bytes: &[u8], count: u32, builder: &mut Builder) -> (Vec<Token>, usize) {
    let mut tokens = Vec::new();
    let mut pos = 4usize;

    for index in 0..count.min(MAX_TOKENS) {
        let Some(len) = rdr::u32_at(bytes, pos) else {
            builder.warn(format!("第 {index} 条令牌的长度字段越界"));
            break;
        };
        let len = len as usize;
        if len == 0 || len > MAX_TOKEN_BYTES || pos + 4 + len > bytes.len() {
            builder.warn(format!("第 {index} 条令牌长度异常：{len}"));
            break;
        }

        let raw = &bytes[pos + 4..pos + 4 + len];
        let body = raw.strip_suffix(&[0u8]).unwrap_or(raw);
        let text = String::from_utf8_lossy(body).into_owned();
        tokens.push(Token {
            text,
            offset: pos as u64,
        });
        pos += 4 + len;
    }

    if count > MAX_TOKENS {
        builder.truncate();
    }

    (tokens, pos)
}

/// 把令牌流还原成文档树；遇到不符合语法的令牌只告警，不panic。
fn parse_document(tokens: &[Token], parent: u32, builder: &mut Builder) {
    let mut cursor = 0usize;

    // 首个令牌按约定是 ROOT。
    if tokens.first().map(|token| token.text.as_str()) != Some("ROOT") {
        builder.warn("首个令牌不是 ROOT");
        mark_root(builder, parent);
    } else {
        cursor = 1;
    }

    parse_body(tokens, &mut cursor, parent, builder);
}

/// 解析 `ROOT` 之后的 `{` 块（或直接以属性开始）。
fn parse_body(tokens: &[Token], cursor: &mut usize, parent: u32, builder: &mut Builder) {
    let braced = tokens
        .get(*cursor)
        .is_some_and(|token| token.text == "{");
    if braced {
        *cursor += 1;
    }

    while let Some(token) = tokens.get(*cursor) {
        if token.text == "}" {
            *cursor += 1;
            if !braced {
                builder.warn("出现未配对的 }");
            }
            return;
        }
        if token.text == "{" {
            builder.warn("出现未命名的 {，跳过");
            *cursor += 1;
            continue;
        }

        if token.text.starts_with('@') {
            parse_node(tokens, cursor, parent, builder);
        } else {
            parse_attribute(tokens, cursor, parent, builder);
        }
    }

    if braced {
        builder.warn("块未闭合即到达令牌末尾");
    }
}

/// `@TYPE ( 标签 ) { ... }`；标签括号可选。
fn parse_node(tokens: &[Token], cursor: &mut usize, parent: u32, builder: &mut Builder) {
    let token = &tokens[*cursor];
    let kind = token.text.clone();
    let offset = token.offset;
    *cursor += 1;

    let label = collect_group(tokens, cursor);
    let name = match label.as_deref() {
        Some(value) if !value.is_empty() => format!("{kind} · {value}"),
        _ => kind.clone(),
    };

    let node = builder.group(parent, Some(&kind), name);
    let _ = offset;

    match tokens.get(*cursor) {
        Some(token) if token.text == "{" => parse_body(tokens, cursor, node, builder),
        _ => {
            builder.warn(format!("{kind} 之后缺少 {{"));
        }
    }
}

/// `名字 ( 值 ) ;`
fn parse_attribute(tokens: &[Token], cursor: &mut usize, parent: u32, builder: &mut Builder) {
    let token = &tokens[*cursor];
    let key = token.text.clone();
    let offset = token.offset;
    *cursor += 1;

    let Some(value) = collect_group(tokens, cursor) else {
        builder.warn(format!("属性 {key} 缺少值括号"));
        return;
    };

    if tokens.get(*cursor).is_some_and(|token| token.text == ";") {
        *cursor += 1;
    } else {
        builder.warn(format!("属性 {key} 缺少分号"));
    }

    builder.text(parent, &key, key.clone(), value, Some(offset));
}

/// 读取 `( a , b , "c" )`：丢掉逗号，字符串去掉引号，其余原样，用 `, ` 连接。
fn collect_group(tokens: &[Token], cursor: &mut usize) -> Option<String> {
    if tokens.get(*cursor).map(|token| token.text.as_str()) != Some("(") {
        return None;
    }
    *cursor += 1;

    let mut parts = Vec::new();
    while let Some(token) = tokens.get(*cursor) {
        match token.text.as_str() {
            ")" => {
                *cursor += 1;
                return Some(parts.join(", "));
            }
            "," => {
                *cursor += 1;
            }
            "(" | "{" | "}" => {
                // 值里出现嵌套括号：不猜语义，原样收进值文本避免死循环。
                parts.push(token.text.clone());
                *cursor += 1;
            }
            raw => {
                parts.push(unquote(raw));
                *cursor += 1;
            }
        }
    }

    None
}

fn unquote(raw: &str) -> String {
    raw.strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(raw)
        .to_string()
}

fn mark_root(builder: &mut Builder, id: u32) {
    builder.mark(id, DocMark::Warn);
}
