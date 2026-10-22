//! 平台 idtag：`.v55` 顶层 `id3` 里的游戏名 → Quad 顶层 `tag` 字段。
//!
//! 逐行移植 PHP `class-quadfile.inc` 的 `load_idtag` + `idtag_decode` +
//! `idtag_decode_token`（ADR-0004：真值是 PHP）。整条链路刻意保持「先拼 JSON 文本、
//! 再解析」的两段式，因为 PHP 正是这么做的：
//!
//! 1. 文本里的换行被替换成 `" \x00 "`，于是每个**有值的行**以一个 NUL 结束；
//! 2. 分词器按词性（键 / 值）把词追进当前 `key` / `val`，遇到 NUL 就吐出一条 `"键" : "值"`；
//! 3. `{\n}` 这类**空容器**会产出键为空串的 `"" : { }`；
//! 4. 于是 `Copyright :`（空值）后面紧跟着的 `Credit` 会被当成**上一个键**的值，
//!    生成 `"" : "Credit"`——同一键出现两次，`json_decode` 取后写入者。
//!
//! 这些「怪癖」都是 PHP 的行为，必须原样复现：测试 `logo_tag_matches_php` 就是拿
//! Unicorn Overlord 的实测输出当锚点。
//!
//! 数据表按 `dirname(脚本)/idtag/*<游戏名>*.txt` 的匹配规则内嵌；本轮只收录 12 个
//! v72/v76/v77 档案（见 [`super::profile`]）能引用到的 5 个文件（ADR-0004 ver 范围）。

use serde_json::Value;

/// 内嵌的 idtag 原文（抄自 `rip-vanillaware/idtag/`，逐字节一致）：`(文件名, 内容)`。
/// 文件名带年份前缀，按 PHP 的 `.*<游戏名>.*\.txt$`（大小写不敏感）做子串匹配。
const FILES: [(&str, &str); 5] = [
    ("2007 odin sphere.txt", include_str!("../../../../spec/idtag/2007 odin sphere.txt")),
    (
        "2007 grim grimoire.txt",
        include_str!("../../../../spec/idtag/2007 grim grimoire.txt"),
    ),
    ("2013 dragon crown.txt", include_str!("../../../../spec/idtag/2013 dragon crown.txt")),
    ("2019 13 sentinels.txt", include_str!("../../../../spec/idtag/2019 13 sentinels.txt")),
    (
        "2024 unicorn overlord.txt",
        include_str!("../../../../spec/idtag/2024 unicorn overlord.txt"),
    ),
];

/// 按游戏名找 idtag 并解码；找不到（或解码失败）返回 `None`。
///
/// 对应 PHP `quadfile::load_idtag`：`tool::find` 取**目录顺序里的首个匹配**，
/// 故这里也按 [`FILES`] 的自然顺序取首个命中。
pub fn lookup(game: &str) -> Option<Value> {
    let needle = game.to_lowercase();
    let (_, raw) = FILES
        .iter()
        .find(|(name, _)| name.to_lowercase().contains(&needle))?;
    decode(raw)
}

/// 对应 PHP `quadfile::idtag_decode`。
///
/// 返回 `{"tag": {…}}`；拼出的文本若不合法（例如值里含未转义的引号）则返回 `None`。
pub fn decode(raw: &str) -> Option<Value> {
    // `str_replace("\n", " \x00 ", $idtag." \n")`
    let mut text = String::with_capacity(raw.len() + 8);
    text.push_str(raw);
    text.push_str(" \n");
    let text = text.replace('\n', " \u{0000} ");

    // `preg_split('|[\s]+|', trim($idtag))`：先在两端剪掉 PHP `trim` 的字符集
    // （空白与 NUL），再按空白段切词；NUL 不是空白，故自己单独成一个词。
    let trimmed = text.trim_matches(is_trim_char);
    let tokens: Vec<&str> = trimmed
        .split(|c: char| c.is_whitespace())
        .filter(|token| !token.is_empty())
        .collect();

    let mut index = 0usize;
    let body = decode_token(&tokens, &mut index, true);
    let json = format!("{{ \"tag\" : {{ {body} }} }}");
    serde_json::from_str(&json).ok()
}

/// PHP `trim` 的默认字符集：`" \t\n\r\0\x0B"`（注意含 NUL，但不含 `\x0C`）。
fn is_trim_char(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0000}' | '\u{000B}')
}

/// 对应 PHP `quadfile::idtag_decode_token`：把词流摊成一段 JSON 片段，
/// 形如 ` "键" : "值" , "键" : [ … ] , "键" : { … }`。
///
/// `tokens` / `index` 就是 PHP 里那个**按引用共享**的 `$token` 数组：
/// 递归调用会继续消耗同一段流，`]` / `}` 由子层消费。
fn decode_token(tokens: &[&str], index: &mut usize, has_key: bool) -> String {
    let mut tmp: Vec<String> = Vec::new();
    let mut key: Vec<&str> = Vec::new();
    let mut val: Vec<&str> = Vec::new();
    let mut is_key = has_key;

    while *index < tokens.len() {
        let token = tokens[*index];
        *index += 1;
        match token {
            "[" => {
                let tk = key.join(" ");
                let tv = decode_token(tokens, index, false);
                tmp.push(format!(" \"{tk}\" : [ {tv} ] "));
                key.clear();
            }
            "{" => {
                let tk = key.join(" ");
                let tv = decode_token(tokens, index, true);
                tmp.push(format!(" \"{tk}\" : {{ {tv} }} "));
                key.clear();
            }
            // `]` / `}`：PHP 用 `break 2` 同时跳出 switch 与 while，即直接返回。
            "]" | "}" => break,
            "\u{0000}" => {
                let tk = key.join(" ");
                let tv = val.join(" ");
                key.clear();
                val.clear();
                // 空值直接丢弃（`Copyright :` 这类），且**不改** `$is_key`。
                if tv.is_empty() {
                    continue;
                }
                if has_key {
                    tmp.push(format!(" \"{tk}\" : \"{tv}\" "));
                    is_key = true;
                } else {
                    tmp.push(format!(" \"{tv}\" "));
                    is_key = false;
                }
            }
            ":" => is_key = false,
            _ => {
                if is_key {
                    key.push(token);
                } else {
                    val.push(token);
                }
            }
        }
    }

    tmp.join(" , ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Unicorn Overlord 的实测锚点：`Copyright` 空值导致其后 `Credit` 被吞，
    /// 生成重复键 `""`，`json_decode` 保留后写入的 `"Credit"`。
    #[test]
    fn logo_tag_matches_php() {
        let tag = lookup("unicorn overlord").expect("unicorn overlord");
        assert_eq!(
            tag,
            json!({
                "tag": {
                    "Game": {
                        "ps4": "Unicorn Overlord",
                        "ps5": "Unicorn Overlord",
                        "switch": "Unicorn Overlord",
                        "xbsx": "Unicorn Overlord",
                    },
                    "Year": "2024",
                    "": "Credit",
                    "ripby": "Vanillaware Ripping Project",
                    "tagby": "-",
                    "comment": [],
                }
            })
        );
    }

    /// 非空值走列表分支：`Copyright [ … ]` 要摊成字符串数组。
    #[test]
    fn array_and_object_nesting() {
        let tag = lookup("grim grimoire").expect("grim grimoire");
        let root = tag.get("tag").expect("tag");
        assert_eq!(root["Year"], json!("2007"));
        assert_eq!(
            root["Copyright"],
            json!(["2007 VANILLAWARE Ltd.", "NIPPON ICHI SOFTWARE INC."])
        );
        assert_eq!(root["Credit"]["Character Designers"], json!(["Kouichi Maenou", "George Kamitani"]));
        assert_eq!(root["comment"], json!([]));
    }

    /// 名字里带冒号/撇号的值要能整体读出。
    #[test]
    fn punctuation_inside_values() {
        let tag = lookup("13 sentinels").expect("13 sentinels");
        assert_eq!(
            tag["tag"]["Game"]["ps4"],
            json!("13 Sentinels: Aegis Rim")
        );
        let dragon = lookup("dragon crown").expect("dragon crown");
        assert_eq!(dragon["tag"]["Game"]["vita"], json!("Dragon's Crown"));
    }
}
