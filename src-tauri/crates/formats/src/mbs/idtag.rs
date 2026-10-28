use serde_json::Value;

const FILES: [(&str, &str); 5] = [
    (
        "2007 odin sphere.txt",
        include_str!("../../../../../spec/idtag/2007 odin sphere.txt"),
    ),
    (
        "2007 grim grimoire.txt",
        include_str!("../../../../../spec/idtag/2007 grim grimoire.txt"),
    ),
    (
        "2013 dragon crown.txt",
        include_str!("../../../../../spec/idtag/2013 dragon crown.txt"),
    ),
    (
        "2019 13 sentinels.txt",
        include_str!("../../../../../spec/idtag/2019 13 sentinels.txt"),
    ),
    (
        "2024 unicorn overlord.txt",
        include_str!("../../../../../spec/idtag/2024 unicorn overlord.txt"),
    ),
];

pub fn lookup(game: &str) -> Option<Value> {
    let needle = game.to_lowercase();
    let (_, raw) = FILES
        .iter()
        .find(|(name, _)| name.to_lowercase().contains(&needle))?;
    decode(raw)
}

pub fn decode(raw: &str) -> Option<Value> {
    // `str_replace("\n", " \x00 ", $idtag." \n")`
    let mut text = String::with_capacity(raw.len() + 8);
    text.push_str(raw);
    text.push_str(" \n");
    let text = text.replace('\n', " \u{0000} ");

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

fn is_trim_char(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0000}' | '\u{000B}')
}

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
            "]" | "}" => break,
            "\u{0000}" => {
                let tk = key.join(" ");
                let tv = val.join(" ");
                key.clear();
                val.clear();
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
