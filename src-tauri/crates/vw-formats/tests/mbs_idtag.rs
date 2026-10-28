use vw_formats::mbs::idtag::*;
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
    assert_eq!(
        root["Credit"]["Character Designers"],
        json!(["Kouichi Maenou", "George Kamitani"])
    );
    assert_eq!(root["comment"], json!([]));
}

/// 名字里带冒号/撇号的值要能整体读出。
#[test]
fn punctuation_inside_values() {
    let tag = lookup("13 sentinels").expect("13 sentinels");
    assert_eq!(tag["tag"]["Game"]["ps4"], json!("13 Sentinels: Aegis Rim"));
    let dragon = lookup("dragon crown").expect("dragon crown");
    assert_eq!(dragon["tag"]["Game"]["vita"], json!("Dragon's Crown"));
}
