use formats::document::scb::*;

fn encode(tokens: &[&str]) -> Vec<u8> {
    let mut out = (tokens.len() as u32).to_le_bytes().to_vec();
    for token in tokens {
        let mut raw = token.as_bytes().to_vec();
        raw.push(0);
        out.extend_from_slice(&(raw.len() as u32).to_le_bytes());
        out.extend_from_slice(&raw);
    }
    out
}

#[test]
fn reads_tokens_and_builds_tree() {
    let bytes = encode(&[
        "ROOT",
        "{",
        "@MONSTER",
        "(",
        "\"Kurabe00\"",
        ")",
        "{",
        "Name",
        "(",
        "\"Sattel Jūrō\"",
        ")",
        ";",
        "Scale",
        "(",
        "1",
        ",",
        "2",
        ")",
        ";",
        "}",
        "}",
    ]);

    let arena = build(&bytes);
    assert!(arena.total_nodes() > 5);
    let doc = arena.doc(0, String::new(), String::new(), String::new(), 0, 0);
    let check = doc
        .checks
        .iter()
        .find(|item| item.id == "scb.eof")
        .expect("eof check");
    assert!(check.ok, "the token stream must end exactly at EOF");
}

#[test]
fn truncated_input_never_panics() {
    let bytes = encode(&[
        "ROOT", "{", "@PAGE", "(", "\"p\"", ")", "{", "Text", "(", "\"x\"", ")", ";", "}", "}",
    ]);
    for len in 0..bytes.len() {
        let arena = build(&bytes[..len]);
        assert!(arena.total_nodes() > 0);
    }
}
