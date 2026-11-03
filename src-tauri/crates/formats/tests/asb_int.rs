use formats::document::asb::{build, AsbModule, Reader};

fn encode(value: i64) -> Vec<u8> {
    let negative = value < 0;
    let magnitude = value.unsigned_abs();
    let sign = if negative { 0x80 } else { 0 };
    let (prefix, bytes) = if magnitude < 0x40 {
        (0, 0)
    } else if magnitude <= 0xff {
        (0x40, 1)
    } else if magnitude <= 0xffff {
        (0x60, 2)
    } else if magnitude <= 0x1ff_ffff {
        (0x70, 3)
    } else if magnitude <= 0x1_ffff_ffff {
        (0x78, 4)
    } else if magnitude <= 0x3_ffff_ffff {
        (0x7c, 5)
    } else if magnitude <= 0x7f_ffff_ffff {
        (0x7e, 6)
    } else {
        (0x7f, 8)
    };
    if bytes == 0 {
        return vec![magnitude as u8 | sign];
    }
    let high = match bytes {
        1 => (magnitude >> 8) as u8 & 0x1f,
        2 => (magnitude >> 16) as u8 & 0x1f,
        3 => (magnitude >> 24) as u8 & 0x0f,
        4 => (magnitude >> 32) as u8 & 0x07,
        5 => (magnitude >> 40) as u8 & 0x03,
        6 => (magnitude >> 48) as u8 & 0x01,
        _ => 0,
    };
    let mut out = vec![prefix | high | sign];
    for shift in (0..bytes).rev() {
        out.push((magnitude >> (shift * 8)) as u8);
    }
    out
}

fn read_int(bytes: &[u8]) -> i64 {
    Reader::new(bytes).enc_int().expect("enc_int")
}

#[test]
fn enc_int_round_trips() {
    for value in [0, 1, -1, 42, -42, 0x3f, -0x3f, 80, -80, 0xffff, 0x7fff_ffff] {
        let bytes = encode(value);
        assert_eq!(read_int(&bytes), value, "value {value} / {bytes:?}");
    }
    let mut raw = vec![0x7f];
    raw.extend_from_slice(&[0, 0, 0, 0, 0, 0, 0, 9]);
    assert_eq!(read_int(&raw), 9);
}

#[test]
fn count_budget_rejects_oversized_tables() {
    let bytes = [0x40, 0xff];
    let mut reader = Reader::new(&bytes);
    let error = reader.take_count("test").expect_err("must reject");
    assert!(error.contains("exceeds remaining"), "{error}");
}

#[derive(Default)]
struct ModuleBuilder {
    out: Vec<u8>,
}

impl ModuleBuilder {
    fn int(&mut self, value: i64) {
        self.out.extend(encode(value));
    }

    fn byte(&mut self, value: u8) {
        self.out.push(value);
    }

    fn text(&mut self, value: &str) {
        self.int((value.len() as i64) << 1);
        self.out.extend(value.as_bytes());
    }

    fn void_type(&mut self) {
        self.int(0);
        self.int(80);
        self.byte(0);
    }

    fn signature(&mut self, name: &str) {
        self.text(name);
        self.void_type();
        self.int(0);
        self.int(1);
        self.int(0);
        self.text("");
    }

    fn debug_body(&mut self) {
        self.int(0);
        self.int(0);
        self.int(0);
        self.text("");
        self.int(0);
        self.int(0);
    }
}

fn minimal_module(ops: &[(u8, &[i64])], used_funcs: usize) -> Vec<u8> {
    let mut b = ModuleBuilder::default();
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(1);
    b.byte(b'f');
    b.signature("main");
    b.byte(0);
    b.int(ops.len() as i64);
    for (op, args) in ops {
        b.byte(*op);
        for arg in *args {
            b.int(*arg);
        }
    }
    b.int(0);
    b.debug_body();
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(0);
    b.int(used_funcs as i64);
    for _ in 0..used_funcs {
        b.byte(b'n');
    }
    b.int(0);
    b.int(0);
    b.int(0);
    b.out
}

#[test]
fn synthetic_module_parses_and_decompiles() {
    let bytes = minimal_module(&[(2, &[42]), (10, &[0])], 0);
    let doc = build(&bytes).expect("doc");
    let check = doc
        .checks()
        .iter()
        .find(|check| check.id == "asb.eof")
        .expect("eof check");
    assert!(check.ok);

    let module = AsbModule::parse(&bytes).expect("parse");
    let asm = module.disassemble(None);
    assert_eq!(asm.functions.len(), 1);
    assert_eq!(
        asm.lines
            .iter()
            .map(|line| line.op.as_str())
            .collect::<Vec<_>>(),
        ["PshC4", "RET"]
    );
    assert_eq!(asm.lines[0].target, None);

    let pseudo = module.pseudo(None);
    assert!(pseudo.text.contains("return;"), "{}", pseudo.text);
    assert!(pseudo.text.contains("main("), "{}", pseudo.text);
}

#[test]
fn dangling_call_target_is_reported() {
    let bytes = minimal_module(&[(9, &[0]), (10, &[0])], 0);
    let doc = build(&bytes).expect("doc");
    let check = doc
        .checks()
        .iter()
        .find(|check| check.id == "asb.refs")
        .expect("refs check");
    assert!(!check.ok);
    assert!(
        check
            .detail
            .as_deref()
            .is_some_and(|text| text.contains("uf[0]")),
        "{:?}",
        check.detail
    );
}
