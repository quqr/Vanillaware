//! ASB（AngelScript 2.34.0 asCWriter）模块结构视图。
//!
//! 逐行复刻 `uo_asb.py` 的 Reader：解析全部声明与函数调试信息以精确到达 EOF；
//! 字节码只保留指令数，不在此展开（反汇编是后续查看器）。

use super::arena::{Builder, DocArena, DocMark};
use super::asb_ops::{ArgKind, ASB_OPS};

const ASOBJ_SCRIPT_OBJECT: u32 = 1 << 21;
const ASOBJ_SHARED: u32 = 1 << 22;
const ASOBJ_ENUM: u32 = 1 << 26;
const ASOBJ_TYPEDEF: u32 = 1 << 28;
const TT_IDENTIFIER: i64 = 5;
const MAX_ITEMS: u32 = 2048;

#[derive(Clone, Debug)]
struct DataType {
    token: i64,
    type_info: Option<TypeInfo>,
    bits: u8,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum TypeInfo {
    List(Box<Option<TypeInfo>>),
    Template {
        name: String,
        namespace: String,
        subs: Vec<TemplateSub>,
    },
    Subtype(String),
    Object { name: String, namespace: String },
    Funcdef {
        name: String,
        parent: Box<Option<TypeInfo>>,
    },
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum TemplateSub {
    Sub(DataType),
    Tok(i64),
}

#[derive(Debug, Default)]
struct TypeDecl {
    name: String,
    size: i64,
    flags: u32,
    namespace: String,
    external: bool,
    values: Vec<(String, i64)>,
    alias: i64,
    derived: String,
    interface: bool,
    props: usize,
    methods: usize,
    ctors: usize,
    vft: usize,
    has_destructor: bool,
}

#[derive(Debug, Default)]
struct FuncInfo {
    name: String,
    ret: String,
    params: usize,
    ftype: i64,
    bits: u8,
    bytecode: usize,
    var_space: i64,
    section: String,
    declared_at: i64,
    lines: usize,
    vars: usize,
    param_names: usize,
    vft_index: i64,
}

#[derive(Debug, Default)]
struct Module {
    strip_debug: bool,
    enums: Vec<TypeDecl>,
    classes: Vec<TypeDecl>,
    typedefs: Vec<TypeDecl>,
    globals: Vec<(String, String)>,
    functions: Vec<FuncInfo>,
    global_funcs: Vec<FuncInfo>,
    binds: Vec<(String, String)>,
    strings: Vec<String>,
    funcdefs: usize,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    strings: Vec<String>,
    dtypes: Vec<DataType>,
    strip_debug: bool,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            pos: 0,
            strings: Vec::new(),
            dtypes: Vec::new(),
            strip_debug: false,
        }
    }

    fn read(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self.pos.checked_add(count).ok_or("offset overflow")?;
        let slice = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| format!("read {count} at {:#x}/{:#x}", self.pos, self.bytes.len()))?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.read(1)?[0])
    }

    fn u32(&mut self) -> Result<u32, String> {
        let raw = self.read(4)?;
        Ok(u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]))
    }

    fn be(&mut self, count: usize) -> Result<u64, String> {
        let raw = self.read(count)?;
        Ok(raw.iter().fold(0u64, |acc, &byte| (acc << 8) | byte as u64))
    }

    fn enc_int(&mut self) -> Result<i64, String> {
        let mut byte = self.u8()?;
        let negative = byte & 0x80 != 0;
        byte &= 0x7f;
        let value = if byte == 0x7f {
            self.be(8)? as i64
        } else if byte & 0x7e == 0x7e {
            ((byte as u64 & 0x01) << 48 | self.be(6)?) as i64
        } else if byte & 0x7c == 0x7c {
            ((byte as u64 & 0x03) << 40 | self.be(5)?) as i64
        } else if byte & 0x78 == 0x78 {
            ((byte as u64 & 0x07) << 32 | self.be(4)?) as i64
        } else if byte & 0x70 == 0x70 {
            ((byte as u64 & 0x0f) << 24 | self.be(3)?) as i64
        } else if byte & 0x60 == 0x60 {
            ((byte as u64 & 0x1f) << 16 | self.be(2)?) as i64
        } else if byte & 0x40 == 0x40 {
            ((byte as u64 & 0x1f) << 8 | self.be(1)?) as i64
        } else {
            (byte & 0x3f) as i64
        };
        Ok(if negative { -value } else { value })
    }

    fn string(&mut self) -> Result<String, String> {
        let value = self.enc_int()?;
        if value & 1 != 0 {
            let index = (value >> 1) as usize;
            return Ok(self.strings.get(index).cloned().unwrap_or_default());
        }
        let len = (value >> 1) as usize;
        let text = if len == 0 {
            String::new()
        } else {
            String::from_utf8_lossy(self.read(len)?).into_owned()
        };
        self.strings.push(text.clone());
        Ok(text)
    }

    fn type_info(&mut self) -> Result<Option<TypeInfo>, String> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(None),
            b'l' => {
                let sub = self.type_info()?;
                Ok(Some(TypeInfo::List(Box::new(sub))))
            }
            b'a' => {
                let name = self.string()?;
                let namespace = self.string()?;
                let count = self.enc_int()?;
                let mut subs = Vec::new();
                for _ in 0..count.max(0) {
                    let kind = self.u8()?;
                    match kind {
                        b's' => subs.push(TemplateSub::Sub(self.data_type()?)),
                        b't' => subs.push(TemplateSub::Tok(self.enc_int()?)),
                        other => return Err(format!("template sub tag {other:?}")),
                    }
                }
                Ok(Some(TypeInfo::Template { name, namespace, subs }))
            }
            b's' => Ok(Some(TypeInfo::Subtype(self.string()?))),
            b'o' => {
                let name = self.string()?;
                let namespace = self.string()?;
                Ok(Some(TypeInfo::Object { name, namespace }))
            }
            b'c' => {
                let name = self.string()?;
                let parent = self.type_info()?;
                Ok(Some(TypeInfo::Funcdef { name, parent: Box::new(parent) }))
            }
            other => Err(format!("type_info tag {other:?} at {:#x}", self.pos - 1)),
        }
    }

    fn data_type(&mut self) -> Result<DataType, String> {
        let value = self.enc_int()?;
        if value != 0 {
            let index = (value - 1) as usize;
            return self
                .dtypes
                .get(index)
                .cloned()
                .ok_or_else(|| format!("dtype cache {index} 越界"));
        }
        let token = self.enc_int()?;
        let type_info = if token == TT_IDENTIFIER {
            self.type_info()?
        } else {
            None
        };
        let bits = self.u8()?;
        let dtype = DataType { token, type_info, bits };
        self.dtypes.push(dtype.clone());
        Ok(dtype)
    }

    fn func_signature(&mut self) -> Result<(FuncInfo, bool), String> {
        let name = self.string()?;
        if name == "$dlgte" {
            return Ok((
                FuncInfo {
                    name,
                    ..Default::default()
                },
                true,
            ));
        }
        let ret = self.data_type()?;
        let param_count = self.enc_int()?.max(0) as usize;
        for _ in 0..param_count {
            self.data_type()?;
        }
        if param_count > 0 {
            let inout = self.enc_int()?.max(0);
            for _ in 0..inout {
                self.enc_int()?;
            }
        }
        let ftype = self.enc_int()?;
        if param_count > 0 {
            let defaults = self.enc_int()?.max(0);
            for _ in 0..defaults {
                self.string()?;
            }
        }
        let obj = self.type_info()?;
        if obj.is_some() {
            self.u8()?;
        } else if ftype == 4 {
            let tag = self.u8()?;
            if tag == b'n' {
                self.string()?;
            } else {
                self.type_info()?;
            }
        } else {
            self.string()?;
        }
        Ok((
            FuncInfo {
                name,
                ret: dtype_brief(&ret),
                params: param_count,
                ftype,
                ..Default::default()
            },
            false,
        ))
    }

    fn byte_code(&mut self) -> Result<usize, String> {
        let count = self.enc_int()?.max(0) as usize;
        for _ in 0..count {
            let op = self.u8()? as usize;
            let kind = ASB_OPS
                .get(op)
                .map(|(_, kind)| *kind)
                .ok_or_else(|| format!("未知操作码 {op}"))?;
            let args = match kind {
                ArgKind::None => 0,
                ArgKind::One => 1,
                ArgKind::Two => 2,
                ArgKind::Three => 3,
            };
            for _ in 0..args {
                self.enc_int()?;
            }
        }
        Ok(count)
    }

    fn function(&mut self) -> Result<Option<FuncInfo>, String> {
        let tag = self.u8()?;
        match tag {
            0 => Ok(None),
            b'r' => {
                let _index = self.enc_int()?;
                Ok(None)
            }
            b'f' => {
                let (mut info, delegate) = self.func_signature()?;
                if delegate {
                    return Ok(Some(info));
                }
                match info.ftype {
                    1 => {
                        let bits = self.u8()?;
                        info.bits = bits;
                        if bits & 4 != 0 {
                            return Ok(Some(info));
                        }
                        info.bytecode = self.byte_code()?;
                        info.var_space = self.enc_int()?;
                        if bits & 8 != 0 {
                            let count = self.enc_int()?.max(0);
                            for _ in 0..count {
                                self.type_info()?;
                                self.enc_int()?;
                            }
                            if count > 0 {
                                self.enc_int()?;
                            }
                            let count = self.enc_int()?.max(0);
                            for _ in 0..count * 3 {
                                self.enc_int()?;
                            }
                        }
                        if bits & 16 != 0 {
                            let count = self.enc_int()?.max(0);
                            for _ in 0..count * 2 {
                                self.enc_int()?;
                            }
                        }
                        if !self.strip_debug {
                            info.lines = self.enc_int()?.max(0) as usize;
                            for _ in 0..info.lines {
                                self.enc_int()?;
                            }
                            let sections = self.enc_int()?.max(0);
                            for index in 0..sections {
                                if index % 2 == 0 {
                                    self.enc_int()?;
                                } else {
                                    self.string()?;
                                }
                            }
                            info.vars = self.enc_int()?.max(0) as usize;
                            for _ in 0..info.vars {
                                self.enc_int()?;
                                self.enc_int()?;
                                self.string()?;
                                self.data_type()?;
                            }
                            info.section = self.string()?;
                            info.declared_at = self.enc_int()?;
                            info.param_names = self.enc_int()?.max(0) as usize;
                            for _ in 0..info.param_names {
                                self.string()?;
                            }
                        }
                    }
                    2 | 3 => {
                        info.vft_index = self.enc_int()?;
                    }
                    4 => {
                        self.u8()?;
                    }
                    _ => {}
                }
                Ok(Some(info))
            }
            other => Err(format!("function tag {other:?} at {:#x}", self.pos - 1)),
        }
    }

    fn type_declaration(&mut self, decl: &mut TypeDecl, phase: u8) -> Result<(), String> {
        if phase == 1 {
            decl.name = self.string()?;
            decl.size = self.enc_int()?;
            decl.flags = self.u32()?;
            decl.namespace = self.string()?;
            if decl.flags & ASOBJ_SHARED != 0 {
                decl.external = self.u8()? == b'e';
            }
            return Ok(());
        }

        if decl.flags & ASOBJ_SHARED != 0 && decl.external {
            return Ok(());
        }

        if phase == 2 {
            if decl.flags & ASOBJ_ENUM != 0 {
                let count = self.enc_int()?.max(0);
                for _ in 0..count {
                    let name = self.string()?;
                    let raw = self.u32()? as i32;
                    decl.values.push((name, (raw >> 24) as i64));
                }
            } else if decl.flags & ASOBJ_TYPEDEF != 0 {
                decl.alias = self.enc_int()?;
            } else {
                decl.derived = type_name(&self.type_info()?);
                let interfaces = self.enc_int()?.max(0);
                for _ in 0..interfaces {
                    self.type_info()?;
                    if !decl.interface {
                        self.enc_int()?;
                    }
                }
                if !decl.interface {
                    decl.has_destructor = self.function()?.is_some();
                    decl.ctors = self.enc_int()?.max(0) as usize;
                    for _ in 0..decl.ctors {
                        self.function()?;
                        self.function()?;
                    }
                }
                decl.methods = self.enc_int()?.max(0) as usize;
                for _ in 0..decl.methods {
                    self.function()?;
                }
                decl.vft = self.enc_int()?.max(0) as usize;
                for _ in 0..decl.vft {
                    self.function()?;
                }
            }
        } else if phase == 3 {
            let count = self.enc_int()?.max(0);
            decl.props = count as usize;
            for _ in 0..count {
                self.string()?;
                self.data_type()?;
                self.enc_int()?;
            }
        }
        Ok(())
    }

    fn read_module(&mut self) -> Result<Module, String> {
        let mut module = Module {
            strip_debug: self.enc_int()? != 0,
            ..Default::default()
        };
        self.strip_debug = module.strip_debug;

        let enum_count = self.enc_int()?.max(0);
        for _ in 0..enum_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            self.type_declaration(&mut decl, 2)?;
            module.enums.push(decl);
        }

        let class_count = self.enc_int()?.max(0);
        for _ in 0..class_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            decl.interface = decl.flags & ASOBJ_SCRIPT_OBJECT != 0 && decl.size == 0;
            module.classes.push(decl);
        }

        module.funcdefs = self.enc_int()?.max(0) as usize;
        for _ in 0..module.funcdefs {
            self.function()?;
        }

        for decl in &mut module.classes {
            if decl.interface {
                self.type_declaration(decl, 2)?;
            }
        }
        for decl in &mut module.classes {
            if !decl.interface {
                self.type_declaration(decl, 2)?;
            }
        }
        for decl in &mut module.classes {
            if !decl.interface {
                self.type_declaration(decl, 3)?;
            }
        }

        let typedef_count = self.enc_int()?.max(0);
        for _ in 0..typedef_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            self.type_declaration(&mut decl, 2)?;
            module.typedefs.push(decl);
        }

        let global_count = self.enc_int()?.max(0);
        for _ in 0..global_count {
            let name = self.string()?;
            let namespace = self.string()?;
            let dtype = self.data_type()?;
            self.function()?;
            module.globals.push((name, format!("{namespace} {}", dtype_brief(&dtype))));
        }

        let function_count = self.enc_int()?.max(0);
        for _ in 0..function_count {
            if let Some(info) = self.function()? {
                module.functions.push(info);
            }
        }
        let global_func_count = self.enc_int()?.max(0);
        for _ in 0..global_func_count {
            if let Some(info) = self.function()? {
                module.global_funcs.push(info);
            }
        }

        let bind_count = self.enc_int()?.max(0);
        for _ in 0..bind_count {
            let (info, _) = self.func_signature()?;
            let target = self.string()?;
            module.binds.push((info.name, target));
        }

        let used_types = self.enc_int()?.max(0);
        for _ in 0..used_types {
            self.type_info()?;
        }
        let used_type_ids = self.enc_int()?.max(0);
        for _ in 0..used_type_ids {
            self.data_type()?;
        }
        let used_funcs = self.enc_int()?.max(0);
        for _ in 0..used_funcs {
            let tag = self.u8()?;
            if tag != b'n' {
                self.func_signature()?;
            }
        }
        let used_global_props = self.enc_int()?.max(0);
        for _ in 0..used_global_props {
            self.string()?;
            self.string()?;
            self.data_type()?;
            self.u8()?;
        }
        let string_count = self.enc_int()?.max(0);
        for _ in 0..string_count {
            module.strings.push(self.string()?);
        }
        let used_obj_props = self.enc_int()?.max(0);
        for _ in 0..used_obj_props {
            self.type_info()?;
            self.string()?;
        }

        Ok(module)
    }
}

pub fn build(bytes: &[u8]) -> Result<DocArena, String> {
    let mut reader = Reader::new(bytes);
    let module = reader.read_module()?;
    let eof = reader.pos == bytes.len();

    let mut builder = Builder::new();
    let root = builder.root("ASB");
    builder.field(root, "size", bytes.len().to_string());
    builder.field(root, "parsedEnd", format!("0x{:x}", reader.pos));
    builder.field(root, "stripDebug", module.strip_debug.to_string());
    builder.field(root, "enums", module.enums.len().to_string());
    builder.field(root, "classes", module.classes.len().to_string());
    builder.field(root, "funcdefs", module.funcdefs.to_string());
    builder.field(root, "typedefs", module.typedefs.len().to_string());
    builder.field(root, "globals", module.globals.len().to_string());
    builder.field(root, "functions", module.functions.len().to_string());
    builder.field(root, "globalFuncs", module.global_funcs.len().to_string());
    builder.field(root, "binds", module.binds.len().to_string());
    builder.field(root, "strings", module.strings.len().to_string());

    builder.check(
        "asb.eof",
        "模块精确消费到 EOF",
        eof,
        Some(format!("{:#x} vs {:#x}", reader.pos, bytes.len())),
    );
    if !eof {
        builder.mark(root, DocMark::Error);
    }

    build_enums(&mut builder, root, &module.enums);
    build_classes(&mut builder, root, &module.classes);
    build_typedefs(&mut builder, root, &module.typedefs);
    build_globals(&mut builder, root, &module.globals);

    let functions = builder.array(
        root,
        Some("functions"),
        format!("Functions ({})", module.functions.len()),
    );
    for (index, info) in module.functions.iter().enumerate().take(MAX_ITEMS as usize) {
        build_function(&mut builder, functions, index, info);
    }
    let global_funcs = builder.array(
        root,
        Some("globalFuncs"),
        format!("GlobalFuncs ({})", module.global_funcs.len()),
    );
    for (index, info) in module.global_funcs.iter().enumerate().take(MAX_ITEMS as usize) {
        build_function(&mut builder, global_funcs, index, info);
    }
    if module.functions.len() > MAX_ITEMS as usize
        || module.global_funcs.len() > MAX_ITEMS as usize
    {
        builder.truncate();
    }

    let binds = builder.array(root, Some("binds"), format!("Binds ({})", module.binds.len()));
    for (index, (name, target)) in module.binds.iter().enumerate().take(256) {
        let key = index.to_string();
        let node = builder.group(binds, Some(&key), name);
        builder.field(node, "module", target);
    }

    let strings = builder.array(root, Some("strings"), format!("Strings ({})", module.strings.len()));
    for (index, text) in module.strings.iter().enumerate().take(512) {
        let key = format!("s{index}");
        builder.text(strings, &key, format!("string[{index}]"), text, None);
    }

    Ok(builder.finish())
}

fn build_enums(builder: &mut Builder, root: u32, enums: &[TypeDecl]) {
    let array = builder.array(root, Some("enumList"), format!("Enums ({})", enums.len()));
    for (index, decl) in enums.iter().enumerate().take(512) {
        let key = index.to_string();
        let name = if decl.name.is_empty() { "enum" } else { decl.name.as_str() };
        let node = builder.group(array, Some(key.as_str()), name);
        builder.field(node, "values", decl.values.len().to_string());
        builder.field(node, "namespace", decl.namespace.as_str());
        for (value_index, (name, value)) in decl.values.iter().enumerate().take(32) {
            let key = format!("v{value_index}");
            builder.field(node, &key, format!("{name} = {value}"));
        }
    }
}

fn build_classes(builder: &mut Builder, root: u32, classes: &[TypeDecl]) {
    let array = builder.array(root, Some("classList"), format!("Classes ({})", classes.len()));
    for (index, decl) in classes.iter().enumerate().take(512) {
        let key = index.to_string();
        let name = if decl.name.is_empty() { "class" } else { decl.name.as_str() };
        let node = builder.group(array, Some(key.as_str()), name);
        builder.field(node, "interface", decl.interface.to_string());
        builder.field(node, "size", decl.size.to_string());
        builder.field(node, "flags", format!("0x{:x}", decl.flags));
        if !decl.derived.is_empty() {
            builder.field(node, "derivedFrom", decl.derived.as_str());
        }
        builder.field(node, "props", decl.props.to_string());
        builder.field(node, "methods", decl.methods.to_string());
        builder.field(node, "ctors", decl.ctors.to_string());
        builder.field(node, "vft", decl.vft.to_string());
        builder.field(node, "destructor", decl.has_destructor.to_string());
    }
}

fn build_typedefs(builder: &mut Builder, root: u32, typedefs: &[TypeDecl]) {
    let array = builder.array(root, Some("typedefs"), format!("Typedefs ({})", typedefs.len()));
    for (index, decl) in typedefs.iter().enumerate().take(256) {
        let key = index.to_string();
        let name = if decl.name.is_empty() { "typedef" } else { decl.name.as_str() };
        let node = builder.group(array, Some(key.as_str()), name);
        builder.field(node, "aliasToken", decl.alias.to_string());
    }
}

fn build_globals(builder: &mut Builder, root: u32, globals: &[(String, String)]) {
    let array = builder.array(root, Some("globals"), format!("Globals ({})", globals.len()));
    for (index, (name, type_text)) in globals.iter().enumerate().take(512) {
        let key = index.to_string();
        builder.field(array, &key, format!("{name} : {type_text}"));
    }
}

fn build_function(builder: &mut Builder, parent: u32, index: usize, info: &FuncInfo) {
    let key = index.to_string();
    let name = if info.name.is_empty() { format!("function[{index}]") } else { info.name.clone() };
    let node = builder.group(parent, Some(key.as_str()), name);
    builder.field(node, "bits", info.bits.to_string());
    builder.field(node, "ret", info.ret.as_str());
    builder.field(node, "params", info.params.to_string());
    builder.field(node, "ftype", info.ftype.to_string());
    builder.field(node, "bytecode", info.bytecode.to_string());
    builder.field(node, "varSpace", info.var_space.to_string());
    if !info.section.is_empty() {
        builder.field(node, "section", info.section.as_str());
        builder.field(
            node,
            "declaredAt",
            format!("{}:{}", info.declared_at >> 20, info.declared_at & 0xFFFFF),
        );
    }
    builder.field(node, "vars", info.vars.to_string());
    builder.field(node, "lines", info.lines.to_string());
    builder.field(node, "paramNames", info.param_names.to_string());
    if info.vft_index != 0 {
        builder.field(node, "vftIndex", info.vft_index.to_string());
    }
}

fn dtype_brief(dtype: &DataType) -> String {
    let base = match &dtype.type_info {
        Some(TypeInfo::Object { name, .. })
        | Some(TypeInfo::Subtype(name))
        | Some(TypeInfo::Template { name, .. })
        | Some(TypeInfo::Funcdef { name, .. }) => name.clone(),
        Some(TypeInfo::List(_)) => "list".to_string(),
        None => format!("tt{}", dtype.token),
    };
    let mut text = base;
    if dtype.bits & 0x01 != 0 {
        text.push('@');
    }
    if dtype.bits & 0x08 != 0 {
        text = format!("const {text}");
    }
    if dtype.bits & 0x04 != 0 {
        text.push('&');
    }
    text
}

fn type_name(info: &Option<TypeInfo>) -> String {
    match info {
        None => String::new(),
        Some(TypeInfo::List(_)) => "list".into(),
        Some(TypeInfo::Template { name, .. })
        | Some(TypeInfo::Subtype(name))
        | Some(TypeInfo::Object { name, .. })
        | Some(TypeInfo::Funcdef { name, .. }) => name.clone(),
    }
}
