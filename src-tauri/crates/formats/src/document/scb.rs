//! ```text
//! +0x00  u32 tokenCount
//! ```
//! ```text
//! ```

use super::arena::{Builder, DocArena, DocMark};
use super::rdr;

const MAX_TOKENS: u32 = 200_000;
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
        builder.warn("file is shorter than 4 bytes");
        builder.mark(root, DocMark::Error);
        return builder.finish();
    }

    let count = rdr::u32_at(bytes, 0).unwrap_or(0);
    builder.field(root, "tokenCount", count.to_string());

    let (tokens, end) = read_tokens(bytes, count, &mut builder);

    builder.check(
        "scb.eof",
        "the token stream ends exactly at EOF",
        tokens.len() == count.min(MAX_TOKENS) as usize && end == bytes.len(),
        Some(format!("{} tokens, ends at 0x{end:x} / 0x{:x}", tokens.len(), bytes.len())),
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
            builder.warn(format!("token {index} length field out of range"));
            break;
        };
        let len = len as usize;
        if len == 0 || len > MAX_TOKEN_BYTES || pos + 4 + len > bytes.len() {
            builder.warn(format!("token {index} has an invalid length: {len}"));
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

fn parse_document(tokens: &[Token], parent: u32, builder: &mut Builder) {
    let mut cursor = 0usize;

    if tokens.first().map(|token| token.text.as_str()) != Some("ROOT") {
        builder.warn("the first token is not ROOT");
        mark_root(builder, parent);
    } else {
        cursor = 1;
    }

    parse_body(tokens, &mut cursor, parent, builder);
}

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
                builder.warn("unmatched }");
            }
            return;
        }
        if token.text == "{" {
            builder.warn("unnamed { skipped");
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
        builder.warn("a block is still open when the tokens run out");
    }
}

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
            builder.warn(format!("{kind} is not followed by {{"));
        }
    }
}

fn parse_attribute(tokens: &[Token], cursor: &mut usize, parent: u32, builder: &mut Builder) {
    let token = &tokens[*cursor];
    let key = token.text.clone();
    let offset = token.offset;
    *cursor += 1;

    let Some(value) = collect_group(tokens, cursor) else {
        builder.warn(format!("attribute {key} has no value parentheses"));
        return;
    };

    if tokens.get(*cursor).is_some_and(|token| token.text == ";") {
        *cursor += 1;
    } else {
        builder.warn(format!("attribute {key} has no semicolon"));
    }

    builder.text(parent, &key, key.clone(), value, Some(offset));
}

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
