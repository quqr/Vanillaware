use std::sync::Arc;

use serde::Serialize;

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
    Object {
        name: String,
        namespace: String,
    },
    Funcdef {
        name: String,
        parent: Box<Option<TypeInfo>>,
    },
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum TemplateSub {
    Sub(Arc<DataType>),
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

#[derive(Clone, Debug, Default)]
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
    ops: Vec<(u8, Vec<i64>)>,
    var_list: Vec<(i64, String, String)>,
    param_list: Vec<String>,
}

#[derive(Debug, Default)]
#[allow(dead_code)]
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
    bind_sigs: Vec<(FuncInfo, String)>,
    used_funcs: Vec<Option<FuncInfo>>,
    used_types: Vec<Option<TypeInfo>>,
    used_type_ids: Vec<Arc<DataType>>,
    used_global_props: Vec<(String, String)>,
    used_obj_props: Vec<(Option<TypeInfo>, String)>,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    strings: Vec<String>,
    dtypes: Vec<Arc<DataType>>,
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

    /// 读取一个计数值，并要求它不超过剩余字节数。
    /// 每个表项至少消耗 1 字节，因此这个上界足以阻止伪造计数导致的
    /// 巨量预分配或长时间空转。
    fn take_count(&mut self, what: &str) -> Result<usize, String> {
        let value = self.enc_int()?;
        let count = value.max(0) as usize;
        let remaining = self.bytes.len().saturating_sub(self.pos);
        if count > remaining {
            return Err(format!(
                "{what} count {count} exceeds remaining {remaining} bytes at {:#x}",
                self.pos
            ));
        }
        Ok(count)
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
                Ok(Some(TypeInfo::Template {
                    name,
                    namespace,
                    subs,
                }))
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
                Ok(Some(TypeInfo::Funcdef {
                    name,
                    parent: Box::new(parent),
                }))
            }
            other => Err(format!("type_info tag {other:?} at {:#x}", self.pos - 1)),
        }
    }

    fn data_type(&mut self) -> Result<Arc<DataType>, String> {
        let value = self.enc_int()?;
        if value != 0 {
            let index = (value - 1) as usize;
            return self.dtypes.get(index).cloned().ok_or_else(|| {
                format!("dtype cache {index} out of range at {:#x}", self.pos)
            });
        }
        let token = self.enc_int()?;
        let type_info = if token == TT_IDENTIFIER {
            self.type_info()?
        } else {
            None
        };
        let bits = self.u8()?;
        let dtype = Arc::new(DataType {
            token,
            type_info,
            bits,
        });
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
        let param_count = self.take_count("parameter")?;
        for _ in 0..param_count {
            self.data_type()?;
        }
        if param_count > 0 {
            let inout = self.take_count("in/out parameter")?;
            for _ in 0..inout {
                self.enc_int()?;
            }
        }
        let ftype = self.enc_int()?;
        if param_count > 0 {
            let defaults = self.take_count("default argument")?;
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

    fn byte_code(&mut self) -> Result<Vec<(u8, Vec<i64>)>, String> {
        let count = self.take_count("instruction")?;
        let mut out = Vec::with_capacity(count.min(4096));
        for index in 0..count {
            let at = self.pos;
            let op = self.u8()?;
            let kind = ASB_OPS
                .get(op as usize)
                .map(|(_, kind)| *kind)
                .ok_or_else(|| {
                    format!("unknown opcode {op} (instruction {index}) at {at:#x}")
                })?;
            let arg_count = match kind {
                ArgKind::None => 0,
                ArgKind::One => 1,
                ArgKind::Two => 2,
                ArgKind::Three => 3,
            };
            let mut args = Vec::with_capacity(arg_count);
            for _ in 0..arg_count {
                args.push(self.enc_int()?);
            }
            out.push((op, args));
        }
        Ok(out)
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
                        info.ops = self.byte_code().map_err(|error| {
                            format!("function {}: {error}", display_name(&info.name))
                        })?;
                        info.bytecode = info.ops.len();
                        info.var_space = self.enc_int()?;
                        if bits & 8 != 0 {
                            let count = self.take_count("object variable")?;
                            for _ in 0..count {
                                self.type_info()?;
                                self.enc_int()?;
                            }
                            if count > 0 {
                                self.enc_int()?;
                            }
                            let count = self.take_count("object variable type")?;
                            for _ in 0..count.saturating_mul(3) {
                                self.enc_int()?;
                            }
                        }
                        if bits & 16 != 0 {
                            let count = self.take_count("jump table")?;
                            for _ in 0..count.saturating_mul(2) {
                                self.enc_int()?;
                            }
                        }
                        if !self.strip_debug {
                            info.lines = self.take_count("line")?;
                            for _ in 0..info.lines {
                                self.enc_int()?;
                            }
                            let sections = self.take_count("section")?;
                            for index in 0..sections {
                                if index % 2 == 0 {
                                    self.enc_int()?;
                                } else {
                                    self.string()?;
                                }
                            }
                            let var_count = self.take_count("local variable")?;
                            for _ in 0..var_count {
                                let _stack_pos = self.enc_int()?;
                                let offset = self.enc_int()?;
                                let name = self.string()?;
                                let dtype = self.data_type()?;
                                info.var_list.push((offset, name, dtype_brief(&dtype)));
                            }
                            info.vars = info.var_list.len();
                            info.section = self.string()?;
                            info.declared_at = self.enc_int()?;
                            let param_count = self.take_count("parameter name")?;
                            for _ in 0..param_count {
                                info.param_list.push(self.string()?);
                            }
                            info.param_names = info.param_list.len();
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
                let count = self.take_count("enum value")?;
                for _ in 0..count {
                    let name = self.string()?;
                    let raw = self.u32()? as i32;
                    decl.values.push((name, (raw >> 24) as i64));
                }
            } else if decl.flags & ASOBJ_TYPEDEF != 0 {
                decl.alias = self.enc_int()?;
            } else {
                decl.derived = type_name(&self.type_info()?);
                let interfaces = self.take_count("interface")?;
                for _ in 0..interfaces {
                    self.type_info()?;
                    if !decl.interface {
                        self.enc_int()?;
                    }
                }
                if !decl.interface {
                    decl.has_destructor = self.function()?.is_some();
                    decl.ctors = self.take_count("constructor")?;
                    for _ in 0..decl.ctors {
                        self.function()?;
                        self.function()?;
                    }
                }
                decl.methods = self.take_count("method")?;
                for _ in 0..decl.methods {
                    self.function()?;
                }
                decl.vft = self.take_count("virtual method")?;
                for _ in 0..decl.vft {
                    self.function()?;
                }
            }
        } else if phase == 3 {
            let count = self.take_count("property")?;
            decl.props = count;
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

        let enum_count = self.take_count("enum")?;
        for _ in 0..enum_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            self.type_declaration(&mut decl, 2)?;
            module.enums.push(decl);
        }

        let class_count = self.take_count("class")?;
        for _ in 0..class_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            decl.interface = decl.flags & ASOBJ_SCRIPT_OBJECT != 0 && decl.size == 0;
            module.classes.push(decl);
        }

        module.funcdefs = self.take_count("funcdef")?;
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

        let typedef_count = self.take_count("typedef")?;
        for _ in 0..typedef_count {
            let mut decl = TypeDecl::default();
            self.type_declaration(&mut decl, 1)?;
            self.type_declaration(&mut decl, 2)?;
            module.typedefs.push(decl);
        }

        let global_count = self.take_count("global")?;
        for _ in 0..global_count {
            let name = self.string()?;
            let namespace = self.string()?;
            let dtype = self.data_type()?;
            self.function()?;
            module
                .globals
                .push((name, format!("{namespace} {}", dtype_brief(&dtype))));
        }

        let function_count = self.take_count("function")?;
        for index in 0..function_count {
            let info = self
                .function()
                .map_err(|error| format!("function[{index}]: {error}"))?;
            if let Some(info) = info {
                module.functions.push(info);
            }
        }
        let global_func_count = self.take_count("global function")?;
        for index in 0..global_func_count {
            let info = self
                .function()
                .map_err(|error| format!("global function[{index}]: {error}"))?;
            if let Some(info) = info {
                module.global_funcs.push(info);
            }
        }

        let bind_count = self.take_count("bind")?;
        for _ in 0..bind_count {
            let (info, _) = self.func_signature()?;
            let target = self.string()?;
            module.binds.push((info.name.clone(), target.clone()));
            module.bind_sigs.push((info, target));
        }

        let used_types = self.take_count("used type")?;
        for _ in 0..used_types {
            module.used_types.push(self.type_info()?);
        }
        let used_type_ids = self.take_count("used type id")?;
        for _ in 0..used_type_ids {
            module.used_type_ids.push(self.data_type()?);
        }
        let used_funcs = self.take_count("used function")?;
        for _ in 0..used_funcs {
            let tag = self.u8()?;
            if tag == b'n' {
                module.used_funcs.push(None);
            } else {
                module.used_funcs.push(Some(self.func_signature()?.0));
            }
        }
        let used_global_props = self.take_count("used global property")?;
        for _ in 0..used_global_props {
            let name = self.string()?;
            let namespace = self.string()?;
            self.data_type()?;
            self.u8()?;
            module.used_global_props.push((name, namespace));
        }
        let string_count = self.take_count("string")?;
        for _ in 0..string_count {
            module.strings.push(self.string()?);
        }
        let used_obj_props = self.take_count("used object property")?;
        for _ in 0..used_obj_props {
            let owner = self.type_info()?;
            let name = self.string()?;
            module.used_obj_props.push((owner, name));
        }

        Ok(module)
    }
}

/// 一次解析的结果，可被缓存后在多个查看器之间复用。
pub struct AsbModule {
    module: Module,
    file_len: usize,
    parsed_end: usize,
}

impl AsbModule {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let mut reader = Reader::new(bytes);
        let module = reader.read_module()?;
        Ok(Self {
            module,
            file_len: bytes.len(),
            parsed_end: reader.pos,
        })
    }

    pub fn disassemble(&self, function_index: Option<usize>) -> AsmDto {
        disassemble_module(&self.module, function_index)
    }

    pub fn pseudo(&self, function_index: Option<usize>) -> PseudoDto {
        pseudo_module(&self.module, function_index)
    }

    pub fn into_doc(self) -> DocArena {
        build_doc(&self.module, self.file_len, self.parsed_end)
    }
}

pub fn build(bytes: &[u8]) -> Result<DocArena, String> {
    Ok(AsbModule::parse(bytes)?.into_doc())
}

fn build_doc(module: &Module, file_len: usize, parsed_end: usize) -> DocArena {
    let eof = parsed_end == file_len;

    let mut builder = Builder::new();
    let root = builder.root("ASB");
    builder.field(root, "size", file_len.to_string());
    builder.field(root, "parsedEnd", format!("0x{parsed_end:x}"));
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
        "modules consume the file exactly",
        eof,
        Some(format!("{parsed_end:#x} vs {file_len:#x}")),
    );
    if !eof {
        builder.mark(root, DocMark::Error);
    }

    let (refs_ok, refs_detail) = validate_refs(module);
    builder.check(
        "asb.refs",
        "opcode operands reference existing tables",
        refs_ok,
        refs_detail,
    );

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
    for (index, info) in module
        .global_funcs
        .iter()
        .enumerate()
        .take(MAX_ITEMS as usize)
    {
        build_function(&mut builder, global_funcs, index, info);
    }
    if module.functions.len() > MAX_ITEMS as usize || module.global_funcs.len() > MAX_ITEMS as usize
    {
        builder.truncate();
        builder.check(
            "asb.limit",
            "document tree lists at most the first functions",
            false,
            Some(format!(
                "{} functions / {} global functions; the asm viewer still shows all",
                module.functions.len(),
                module.global_funcs.len()
            )),
        );
    }

    let binds = builder.array(
        root,
        Some("binds"),
        format!("Binds ({})", module.binds.len()),
    );
    for (index, (name, target)) in module.binds.iter().enumerate().take(256) {
        let key = index.to_string();
        let node = builder.group(binds, Some(&key), name);
        builder.field(node, "module", target);
    }

    let strings = builder.array(
        root,
        Some("strings"),
        format!("Strings ({})", module.strings.len()),
    );
    for (index, text) in module.strings.iter().enumerate().take(512) {
        let key = format!("s{index}");
        builder.text(strings, &key, format!("string[{index}]"), text, None);
    }

    builder.finish()
}

fn build_enums(builder: &mut Builder, root: u32, enums: &[TypeDecl]) {
    let array = builder.array(root, Some("enumList"), format!("Enums ({})", enums.len()));
    for (index, decl) in enums.iter().enumerate().take(512) {
        let key = index.to_string();
        let name = if decl.name.is_empty() {
            "enum"
        } else {
            decl.name.as_str()
        };
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
    let array = builder.array(
        root,
        Some("classList"),
        format!("Classes ({})", classes.len()),
    );
    for (index, decl) in classes.iter().enumerate().take(512) {
        let key = index.to_string();
        let name = if decl.name.is_empty() {
            "class"
        } else {
            decl.name.as_str()
        };
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
    let array = builder.array(
        root,
        Some("typedefs"),
        format!("Typedefs ({})", typedefs.len()),
    );
    for (index, decl) in typedefs.iter().enumerate().take(256) {
        let key = index.to_string();
        let name = if decl.name.is_empty() {
            "typedef"
        } else {
            decl.name.as_str()
        };
        let node = builder.group(array, Some(key.as_str()), name);
        builder.field(node, "aliasToken", decl.alias.to_string());
    }
}

fn build_globals(builder: &mut Builder, root: u32, globals: &[(String, String)]) {
    let array = builder.array(
        root,
        Some("globals"),
        format!("Globals ({})", globals.len()),
    );
    for (index, (name, type_text)) in globals.iter().enumerate().take(512) {
        let key = index.to_string();
        builder.field(array, &key, format!("{name} : {type_text}"));
    }
}

fn build_function(builder: &mut Builder, parent: u32, index: usize, info: &FuncInfo) {
    let key = index.to_string();
    let name = if info.name.is_empty() {
        format!("function[{index}]")
    } else {
        info.name.clone()
    };
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmFunctionDto {
    pub index: u32,
    pub name: String,
    pub ret: String,
    pub params: u32,
    pub bytecode: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmLineDto {
    pub index: u32,
    pub op: String,
    pub args: Vec<String>,
    pub note: Option<String>,
    /// 跳转类指令的有效目标指令序号；越界或非跳转时为 None。
    pub target: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AsmDto {
    pub functions: Vec<AsmFunctionDto>,
    pub selected: u32,
    pub lines: Vec<AsmLineDto>,
}

pub fn disassemble_file(bytes: &[u8], function_index: Option<usize>) -> Result<AsmDto, String> {
    Ok(AsbModule::parse(bytes)?.disassemble(function_index))
}

fn disassemble_module(module: &Module, function_index: Option<usize>) -> AsmDto {
    let mut functions = Vec::with_capacity(module.functions.len() + module.global_funcs.len());
    for (index, info) in module
        .functions
        .iter()
        .chain(module.global_funcs.iter())
        .enumerate()
    {
        functions.push(AsmFunctionDto {
            index: index as u32,
            name: info.name.clone(),
            ret: info.ret.clone(),
            params: info.params as u32,
            bytecode: info.bytecode as u32,
        });
    }

    if functions.is_empty() {
        return AsmDto {
            functions,
            selected: 0,
            lines: Vec::new(),
        };
    }

    let selected = function_index
        .unwrap_or_else(|| default_function(module))
        .min(functions.len() - 1);
    let info = if selected < module.functions.len() {
        &module.functions[selected]
    } else {
        &module.global_funcs[selected - module.functions.len()]
    };

    AsmDto {
        functions,
        selected: selected as u32,
        lines: disassemble(module, info),
    }
}

/// 默认选中第一个真正带字节码的函数，避免一打开就是空壳。
fn default_function(module: &Module) -> usize {
    module
        .functions
        .iter()
        .chain(module.global_funcs.iter())
        .position(|info| info.bytecode > 0)
        .unwrap_or(0)
}

fn disassemble(module: &Module, info: &FuncInfo) -> Vec<AsmLineDto> {
    let mut lines = Vec::with_capacity(info.ops.len());
    for (index, (op, args)) in info.ops.iter().enumerate() {
        let name = ASB_OPS
            .get(*op as usize)
            .map(|(name, _)| *name)
            .unwrap_or("???");
        let target = jump_target(index, args.first().copied().unwrap_or(0), info.ops.len());
        let note = disassembly_note(module, name, args, target);
        lines.push(AsmLineDto {
            index: index as u32,
            op: name.to_string(),
            args: args.iter().map(i64::to_string).collect(),
            note,
            target: target.map(|value| value as u32),
        });
    }
    lines
}

fn disassembly_note(
    module: &Module,
    op: &str,
    args: &[i64],
    target: Option<usize>,
) -> Option<String> {
    let first = args.first().copied().unwrap_or(-1);
    match op {
        "CALL" | "CALLSYS" | "CALLINTF" | "Thiscall1" => module
            .used_funcs
            .get(first as usize)
            .and_then(|entry| entry.as_ref())
            .map(|sig| format!("{} ({} params)", sig.name, sig.params))
            .or_else(|| Some(format!("uf[{first}]?"))),
        "ALLOC" | "FREE" | "REFCPY" | "RefCpyV" | "OBJTYPE" => {
            let mut note = module
                .used_type_ids
                .get(first as usize)
                .map(|dtype| dtype_brief(dtype))
                .unwrap_or_else(|| format!("ut[{first}]?"));
            if op == "ALLOC" && args.len() > 1 && args[1] > 0 {
                if let Some(Some(ctor)) = module.used_funcs.get((args[1] - 1) as usize) {
                    note.push_str(&format!(" ctor {}", ctor.name));
                }
            }
            Some(note)
        }
        "PGA" | "PshGPtr" | "LDG" | "PshG4" | "LdGRdR4" | "CpyGtoV4" | "CpyVtoG4" | "SetG4" => {
            if first & 1 != 0 {
                let global = (first >> 1) as usize;
                Some(
                    module
                        .used_global_props
                        .get(global)
                        .map(|(name, namespace)| {
                            if namespace.is_empty() {
                                name.clone()
                            } else {
                                format!("{namespace}::{name}")
                            }
                        })
                        .unwrap_or_else(|| format!("gp[{global}]?")),
                )
            } else {
                let string_index = (first >> 1) as usize;
                if matches!(op, "PGA" | "PshGPtr") {
                    module.strings.get(string_index).map(|text| {
                        let short: String = text.chars().take(60).collect();
                        format!("str {short:?}")
                    })
                } else {
                    Some(format!("stridx {string_index}?"))
                }
            }
        }
        "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ" => Some(
            target
                .map(|value| format!("-> {value}"))
                .unwrap_or_else(|| "-> <out of range>".to_string()),
        ),
        "COPY" => module
            .used_type_ids
            .get(first as usize)
            .map(|dtype| dtype_brief(dtype)),
        "CALLBND" => module
            .bind_sigs
            .get(first as usize)
            .map(|(sig, target)| format!("{} from {target}", sig.name)),
        "PshC4" | "PshC8" => Some(first.to_string()),
        "SetV4" | "SetV1" | "SetV2" if args.len() > 1 => {
            Some(format!("v{} = {}", args[0], args[1]))
        }
        "ADDSi" | "LoadThisR" if args.len() >= 2 => {
            let prop = args[0] as usize;
            let type_index = args[1] as usize;
            let prop_name = module
                .used_obj_props
                .get(prop)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| format!("prop[{prop}]"));
            let type_name = module
                .used_type_ids
                .get(type_index)
                .map(|dtype| dtype_brief(dtype))
                .unwrap_or_else(|| format!("ut[{type_index}]"));
            Some(format!("{prop_name} ; {type_name}"))
        }
        "LoadRObjR" | "LoadVObjR" if args.len() >= 3 => {
            let prop = args[1] as usize;
            let type_index = args[2] as usize;
            let prop_name = module
                .used_obj_props
                .get(prop)
                .map(|(_, name)| name.clone())
                .unwrap_or_else(|| format!("prop[{prop}]"));
            let type_name = module
                .used_type_ids
                .get(type_index)
                .map(|dtype| dtype_brief(dtype))
                .unwrap_or_else(|| format!("ut[{type_index}]"));
            Some(format!("{prop_name} ; {type_name}"))
        }
        _ => None,
    }
}

/// 校验字节码操作数是否落在对应的使用表内，并检查跳转目标。
fn validate_refs(module: &Module) -> (bool, Option<String>) {
    struct Checker {
        total: usize,
        samples: Vec<String>,
    }

    impl Checker {
        fn check(&mut self, context: &str, table: &str, value: i64, len: usize) {
            if value < 0 || value as usize >= len {
                self.total += 1;
                if self.samples.len() < 8 {
                    self.samples.push(format!("{context}: {table}[{value}] / {len}"));
                }
            }
        }
    }

    let mut checker = Checker {
        total: 0,
        samples: Vec::new(),
    };
    for (kind, functions) in [
        ("function", &module.functions),
        ("global", &module.global_funcs),
    ] {
        for (fi, info) in functions.iter().enumerate() {
            let context = format!("{kind}[{fi}] {}", display_name(&info.name));
            for (index, (op, args)) in info.ops.iter().enumerate() {
                let name = op_name(*op);
                let first = args.first().copied().unwrap_or(-1);
                let at = format!("{context} #{index} {name}");
                match name {
                    "CALL" | "CALLSYS" | "CALLINTF" | "Thiscall1" => {
                        checker.check(&at, "uf", first, module.used_funcs.len());
                    }
                    "CALLBND" => {
                        checker.check(&at, "bind", first, module.bind_sigs.len());
                    }
                    "PGA" | "PshGPtr" | "LDG" | "PshG4" | "LdGRdR4" | "CpyGtoV4"
                    | "CpyVtoG4" | "SetG4" => {
                        if first & 1 != 0 {
                            checker.check(&at, "gp", first >> 1, module.used_global_props.len());
                        } else {
                            checker.check(&at, "str", first >> 1, module.strings.len());
                        }
                    }
                    "ADDSi" | "LoadThisR" if args.len() >= 2 => {
                        checker.check(&at, "prop", args[0], module.used_obj_props.len());
                        checker.check(&at, "ut", args[1], module.used_type_ids.len());
                    }
                    "LoadRObjR" | "LoadVObjR" if args.len() >= 3 => {
                        checker.check(&at, "prop", args[1], module.used_obj_props.len());
                        checker.check(&at, "ut", args[2], module.used_type_ids.len());
                    }
                    _ if is_jump(name) && jump_target(index, first, info.ops.len()).is_none() => {
                        checker.total += 1;
                        if checker.samples.len() < 8 {
                            checker
                                .samples
                                .push(format!("{at}: jump target out of range"));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let ok = checker.total == 0;
    let detail = if ok {
        None
    } else {
        let mut text = checker.samples.join("; ");
        if checker.total > checker.samples.len() {
            text.push_str(&format!(
                " (+{} more)",
                checker.total - checker.samples.len()
            ));
        }
        Some(text)
    };
    (ok, detail)
}

/// 把相对的跳转参数换算成绝对指令序号；越界时返回 None。
fn jump_target(index: usize, delta: i64, limit: usize) -> Option<usize> {
    let target = (index as i64).checked_add(1)?.checked_add(delta)?;
    if target < 0 || target as usize >= limit {
        None
    } else {
        Some(target as usize)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoDto {
    pub text: String,
}

pub fn pseudo_file(bytes: &[u8], function_index: Option<usize>) -> Result<PseudoDto, String> {
    Ok(AsbModule::parse(bytes)?.pseudo(function_index))
}

fn pseudo_module(module: &Module, function_index: Option<usize>) -> PseudoDto {
    let total = module.functions.len() + module.global_funcs.len();
    if total == 0 {
        return PseudoDto {
            text: String::new(),
        };
    }
    let selected = function_index
        .unwrap_or_else(|| default_function(module))
        .min(total - 1);
    let info = if selected < module.functions.len() {
        &module.functions[selected]
    } else {
        &module.global_funcs[selected - module.functions.len()]
    };
    PseudoDto {
        text: decompile(module, info),
    }
}

fn decompile(module: &Module, info: &FuncInfo) -> String {
    let mut vars = std::collections::HashMap::new();
    for (offset, name, _) in &info.var_list {
        vars.insert(
            *offset,
            if name.is_empty() {
                format!("tmp{offset}")
            } else {
                name.clone()
            },
        );
    }
    let mut params = Vec::new();
    for index in 0..info.params {
        params.push(
            vars.get(&(-(index as i64)))
                .cloned()
                .or_else(|| info.param_list.get(index).cloned())
                .unwrap_or_else(|| format!("arg{index}")),
        );
    }

    let mut leaders = vec![false; info.ops.len()];
    let mut targeted = vec![false; info.ops.len()];
    if !info.ops.is_empty() {
        leaders[0] = true;
    }
    for (index, (op, args)) in info.ops.iter().enumerate() {
        let name = op_name(*op);
        if is_jump(name) {
            if let Some(target) =
                jump_target(index, args.first().copied().unwrap_or(0), info.ops.len())
            {
                targeted[target] = true;
                leaders[target] = true;
            }
        }
        if (is_jump(name) || name == "RET") && index + 1 < info.ops.len() {
            leaders[index + 1] = true;
        }
    }

    let mut out: Vec<String> = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut retreg: Option<String> = None;
    let mut pending_cmp: Option<(String, String)> = None;

    for (index, (op, args)) in info.ops.iter().enumerate() {
        // 比较标志不跨块；只有不可达的死代码才清空残留栈值，
        // 跳转目标处的栈值可能是两条路径的合流结果，必须保留。
        if leaders[index] && index > 0 {
            let after_terminator = matches!(op_name(info.ops[index - 1].0), "RET" | "JMP");
            if after_terminator && !targeted[index] && !stack.is_empty() {
                out.push(format!(
                    "// L{index}: discarded stack [{}]",
                    stack.join(", ")
                ));
                stack.clear();
            }
            pending_cmp = None;
        }
        if targeted[index] {
            out.push(format!("L{index}:"));
        }
        let name = op_name(*op);
        let first = args.first().copied().unwrap_or(-1);
        let var = |offset: i64| {
            vars.get(&offset)
                .cloned()
                .unwrap_or_else(|| format!("v{offset}"))
        };

        match name {
            "PshC4" => stack.push(signed_i32(first).to_string()),
            "PshC8" => stack.push(first.to_string()),
            "PshV4" | "PshVPtr" | "PSF" | "PshV8" | "VAR" => stack.push(var(first)),
            "PGA" | "PshGPtr" | "PshG4" => stack.push(str_or_global(module, first)),
            "PshNull" => stack.push("null".to_string()),
            "PopPtr" | "PopRPtr" => {
                stack.pop();
            }
            "SwapPtr" => {
                let len = stack.len();
                if len >= 2 {
                    stack.swap(len - 1, len - 2);
                }
            }
            "ClrVPtr" => out.push(format!("{} = null;", var(first))),
            "SetV4" if args.len() == 2 => {
                out.push(format!("{} = {};", var(args[0]), signed_i32(args[1])));
            }
            "SetV8" if args.len() == 2 => {
                out.push(format!("{} = {};", var(args[0]), args[1]));
            }
            "SetV1" if args.len() == 2 => {
                out.push(format!("{} = {};", var(args[0]), args[1] & 0xFF));
            }
            "SetV2" if args.len() == 2 => {
                out.push(format!("{} = {};", var(args[0]), args[1] & 0xFFFF));
            }
            "SetG4" if args.len() == 2 => {
                out.push(format!(
                    "{} = {};",
                    str_or_global(module, args[0]),
                    signed_i32(args[1])
                ));
            }
            "CpyVtoV4" | "CpyVtoV8" if args.len() == 2 => {
                out.push(format!("{} = {};", var(args[0]), var(args[1])));
            }
            "CpyVtoR4" | "CpyVtoR8" => retreg = Some(var(first)),
            "CpyRtoV4" | "CpyRtoV8" => {
                out.push(format!(
                    "{} = {};",
                    var(first),
                    retreg.clone().unwrap_or_else(|| "ret".into())
                ));
            }
            "CpyGtoV4" | "LdGRdR4" if args.len() == 2 => {
                out.push(format!(
                    "{} = {};",
                    var(args[0]),
                    str_or_global(module, args[1])
                ));
            }
            "CpyVtoG4" if args.len() == 2 => {
                out.push(format!(
                    "{} = {};",
                    str_or_global(module, args[1]),
                    var(args[0])
                ));
            }
            _ if is_bin_op(name) && args.len() == 3 => {
                out.push(format!(
                    "{} = {} {} {};",
                    var(args[0]),
                    var(args[1]),
                    bin_op(name),
                    var(args[2])
                ));
            }
            _ if is_imm_op(name) && args.len() == 3 => {
                let imm = if name.contains('f') {
                    format_float(args[2])
                } else {
                    args[2].to_string()
                };
                out.push(format!(
                    "{} = {} {} {};",
                    var(args[0]),
                    var(args[1]),
                    bin_op(name),
                    imm
                ));
            }
            _ if is_pow(name) && args.len() == 3 => {
                out.push(format!(
                    "{} = pow({}, {});",
                    var(args[0]),
                    var(args[1]),
                    var(args[2])
                ));
            }
            _ if is_conversion(name) && !args.is_empty() => {
                let dst = var(args[0]);
                let src = if args.len() > 1 {
                    var(args[1])
                } else {
                    dst.clone()
                };
                out.push(format!("{dst} = {name}({src});"));
            }
            _ if is_neg(name) => out.push(format!("{} = -{};", var(first), var(first))),
            "INCi" | "INCf" | "INCi64" | "IncVi" | "INCu" => {
                out.push(format!("++{};", var(first)));
            }
            "DECi" | "DECf" | "DECi64" | "DecVi" | "DECu" => {
                out.push(format!("--{};", var(first)));
            }
            "CMPi" | "CMPu" | "CMPf" | "CMPd" | "CMPi64" | "CMPu64" if args.len() == 2 => {
                pending_cmp = Some((var(args[0]), var(args[1])));
            }
            "CMPIi" | "CMPIu" | "CMPIf" | "CMPIi64" | "CMPIu64" if args.len() == 2 => {
                let imm = if name == "CMPIf" {
                    format_float(args[1])
                } else {
                    args[1].to_string()
                };
                pending_cmp = Some((var(args[0]), imm));
            }
            "NOT" => out.push(format!("{} = !{};", var(first), var(first))),
            "BNOT" | "BNOT64" => out.push(format!("{} = ~{};", var(first), var(first))),
            _ if is_jump(name) => {
                let target = jump_target(index, first, info.ops.len());
                let invalid = target.is_none();
                let label = target
                    .map(|value| format!("L{value}"))
                    .unwrap_or_else(|| {
                        format!(
                            "L{}",
                            (index as i64)
                                .saturating_add(1)
                                .saturating_add(first)
                        )
                    });
                if name == "JMP" {
                    out.push(format!(
                        "{}goto {label};",
                        if invalid { "/* out of range */ " } else { "" }
                    ));
                } else {
                    let condition = match name {
                        "JZ" | "JLowZ" => pending_cmp
                            .take()
                            .map(|(left, right)| format!("{left} == {right}"))
                            .or_else(|| retreg.clone().map(|value| format!("!({value})")))
                            .unwrap_or_else(|| "cond".into()),
                        "JNZ" | "JLowNZ" => pending_cmp
                            .take()
                            .map(|(left, right)| format!("{left} != {right}"))
                            .or_else(|| retreg.clone())
                            .unwrap_or_else(|| "cond".into()),
                        "JS" | "JNS" | "JP" | "JNP" => {
                            let operator = match name {
                                "JS" => "<",
                                "JNS" => ">=",
                                "JP" => ">",
                                _ => "<=",
                            };
                            pending_cmp
                                .take()
                                .map(|(left, right)| format!("{left} {operator} {right}"))
                                .unwrap_or_else(|| "cond".into())
                        }
                        _ => "cond".into(),
                    };
                    out.push(format!(
                        "{}if ({condition}) goto {label};",
                        if invalid { "/* out of range */ " } else { "" }
                    ));
                }
            }
            "RET" => {
                if !is_void(&info.ret) && retreg.is_some() {
                    out.push(format!("return {};", retreg.clone().unwrap()));
                } else {
                    out.push("return;".into());
                }
            }
            "CALL" | "CALLSYS" | "CALLINTF" | "Thiscall1" | "CALLBND" => {
                let target = if name == "CALLBND" {
                    module.bind_sigs.get(first as usize).map(|(sig, _)| sig)
                } else {
                    module
                        .used_funcs
                        .get(first as usize)
                        .and_then(|entry| entry.as_ref())
                };
                let (call_name, param_count, ret) = match target {
                    Some(sig) => (sig.name.clone(), sig.params, sig.ret.clone()),
                    None => (format!("fn{first}?"), 0, String::new()),
                };
                let mut values = Vec::new();
                for _ in 0..param_count {
                    values.push(stack.pop().unwrap_or_else(|| "??".into()));
                }
                values.reverse();
                let call = format!("{call_name}({})", values.join(", "));
                if is_void(&ret) {
                    out.push(format!("{call};"));
                } else {
                    retreg = Some(call);
                }
            }
            "ALLOC" => {
                let type_name = module
                    .used_types
                    .get(first as usize)
                    .and_then(|info| info.as_ref())
                    .map(type_info_name)
                    .unwrap_or_else(|| "obj".into());
                out.push(format!(
                    "/* new {} */",
                    if type_name.is_empty() {
                        "<anonymous>"
                    } else {
                        &type_name
                    }
                ));
            }
            "LoadRObjR" | "LoadVObjR" if args.len() >= 3 => {
                let property = args[1] as usize;
                if let Some((_, name)) = module.used_obj_props.get(property) {
                    stack.push(name.clone());
                }
            }
            "STOREOBJ" => {
                let value = stack.pop().unwrap_or_else(|| "??".into());
                out.push(format!("{} = {};", var(first), value));
            }
            "LOADOBJ" => stack.push(var(first)),
            _ => {
                if !is_noise(name) {
                    out.push(format!("/* {name} {args:?} */"));
                }
            }
        }
    }

    while let Some(value) = stack.pop() {
        out.push(format!("// [stack leftovers] {value}"));
    }

    let mut text = String::new();
    let head_params = params.join(", ");
    text.push_str(&format!(
        "{} {}({head_params})\n",
        info.ret,
        display_name(&info.name)
    ));
    if !info.section.is_empty() {
        text.push_str(&format!(
            "    // {}:{}:{}\n",
            info.section,
            info.declared_at >> 20,
            info.declared_at & 0xFFFFF
        ));
    }
    for line in out {
        text.push_str("    ");
        text.push_str(&line);
        text.push('\n');
    }
    text
}

fn op_name(op: u8) -> &'static str {
    ASB_OPS
        .get(op as usize)
        .map(|(name, _)| *name)
        .unwrap_or("???")
}

fn is_jump(name: &str) -> bool {
    matches!(
        name,
        "JMP" | "JZ" | "JNZ" | "JS" | "JNS" | "JP" | "JNP" | "JLowZ" | "JLowNZ"
    )
}

fn is_bin_op(name: &str) -> bool {
    matches!(
        name,
        "ADDi"
            | "SUBi"
            | "MULi"
            | "DIVi"
            | "MODi"
            | "ADDi64"
            | "SUBi64"
            | "MULi64"
            | "DIVi64"
            | "MODi64"
            | "ADDf"
            | "SUBf"
            | "MULf"
            | "DIVf"
            | "MODf"
            | "ADDd"
            | "SUBd"
            | "MULd"
            | "DIVd"
            | "MODd"
            | "ADDu"
            | "SUBu"
            | "MULu"
            | "DIVu"
            | "MODu"
            | "ADDu64"
            | "SUBu64"
            | "MULu64"
            | "DIVu64"
            | "MODu64"
            | "BAND"
            | "BOR"
            | "BXOR"
            | "BSLL"
            | "BSRL"
            | "BSRA"
            | "BAND64"
            | "BOR64"
            | "BXOR64"
            | "BSLL64"
            | "BSRL64"
            | "BSRA64"
    )
}

fn is_imm_op(name: &str) -> bool {
    matches!(
        name,
        "ADDIi"
            | "SUBIi"
            | "MULIi"
            | "ADDIf"
            | "SUBIf"
            | "MULIf"
            | "ADDIi64"
            | "SUBIi64"
            | "MULIi64"
    )
}

fn bin_op(name: &str) -> &'static str {
    match name {
        "ADDi" | "ADDi64" | "ADDf" | "ADDd" | "ADDu" | "ADDu64" | "ADDIf" | "ADDIi" | "ADDIi64" => {
            "+"
        }
        "SUBi" | "SUBi64" | "SUBf" | "SUBd" | "SUBu" | "SUBu64" | "SUBIf" | "SUBIi" | "SUBIi64" => {
            "-"
        }
        "MULi" | "MULi64" | "MULf" | "MULd" | "MULu" | "MULu64" | "MULIf" | "MULIi" | "MULIi64" => {
            "*"
        }
        "DIVi" | "DIVi64" | "DIVf" | "DIVd" | "DIVu" | "DIVu64" => "/",
        "MODi" | "MODi64" | "MODf" | "MODd" | "MODu" | "MODu64" => "%",
        "BAND" | "BAND64" => "&",
        "BOR" | "BOR64" => "|",
        "BXOR" | "BXOR64" => "^",
        "BSLL" | "BSLL64" => "<<",
        "BSRL" | "BSRL64" => ">>",
        "BSRA" | "BSRA64" => ">>",
        _ => "?",
    }
}

fn is_pow(name: &str) -> bool {
    matches!(
        name,
        "POWi" | "POWu" | "POWf" | "POWd" | "POWdi" | "POWi64" | "POWu64"
    )
}

fn is_conversion(name: &str) -> bool {
    matches!(
        name,
        "iTOf"
            | "fTOi"
            | "uTOf"
            | "fTOu"
            | "sbTOi"
            | "swTOi"
            | "ubTOi"
            | "uwTOi"
            | "iTOb"
            | "iTOw"
            | "Cast"
            | "dTOi"
            | "dTOu"
            | "dTOf"
            | "iTOd"
            | "uTOd"
            | "fTOd"
            | "i64TOi"
            | "uTOi64"
            | "iTOi64"
            | "fTOi64"
            | "dTOi64"
            | "fTOu64"
            | "dTOu64"
            | "i64TOf"
            | "u64TOf"
            | "i64TOd"
            | "u64TOd"
    )
}

fn is_neg(name: &str) -> bool {
    matches!(name, "NEGi" | "NEGf" | "NEGd" | "NEGi64")
}

fn is_noise(name: &str) -> bool {
    matches!(
        name,
        "SUSPEND"
            | "ChkRefS"
            | "ChkNullV"
            | "CHKREF"
            | "ClrVPtr"
            | "SwapPtr"
            | "PopPtr"
            | "PshNull"
            | "RDSPtr"
            | "PshRPtr"
            | "PopRPtr"
            | "GETOBJ"
            | "GETOBJREF"
            | "GETREF"
            | "JitEntry"
            | "AllocMem"
            | "SetListSize"
            | "PshListElmnt"
            | "SetListType"
            | "JMPP"
            | "ObjInfo"
            | "TYPEID"
            | "Cast"
            | "CmpPtr"
            | "LOADOBJ"
            | "STOREOBJ"
            | "REFCPY"
            | "RefCpyV"
            | "FREE"
            | "COPY"
            | "WRTV1"
            | "WRTV2"
            | "WRTV4"
            | "WRTV8"
            | "RDR1"
            | "RDR2"
            | "RDR4"
            | "RDR8"
            | "LDG"
            | "LDV"
            | "PSF"
            | "VAR"
            | "PshV8"
            | "VARDECL"
    )
}

fn str_or_global(module: &Module, value: i64) -> String {
    if value & 1 != 0 {
        let index = (value >> 1) as usize;
        return module
            .used_global_props
            .get(index)
            .map(|(name, namespace)| {
                if namespace.is_empty() {
                    name.clone()
                } else {
                    format!("{namespace}::{name}")
                }
            })
            .unwrap_or_else(|| format!("global{index}"));
    }
    let index = (value >> 1) as usize;
    module
        .strings
        .get(index)
        .map(|text| format!("\"{text}\""))
        .unwrap_or_else(|| format!("str{index}"))
}

fn signed_i32(value: i64) -> i32 {
    (value & 0xFFFF_FFFF) as u32 as i32
}

fn format_float(value: i64) -> String {
    let raw = (value & 0xFFFF_FFFF) as u32;
    format!("{:?}", f32::from_bits(raw))
}

fn display_name(name: &str) -> &str {
    if name.is_empty() { "<anonymous>" } else { name }
}

fn is_void(ret: &str) -> bool {
    let base = ret.split('@').next().unwrap_or(ret).trim();
    base == "void" || base == "?"
}

fn tt_name(token: i64) -> &'static str {
    match token {
        65 => "bool",
        68 => "int",
        69 => "int8",
        70 => "int16",
        71 => "int64",
        75 => "uint",
        76 => "uint8",
        77 => "uint16",
        78 => "uint64",
        79 => "float",
        80 => "void",
        92 => "double",
        _ => "?",
    }
}

fn dtype_brief(dtype: &DataType) -> String {
    let base = match &dtype.type_info {
        Some(TypeInfo::Object { name, .. })
        | Some(TypeInfo::Subtype(name))
        | Some(TypeInfo::Template { name, .. })
        | Some(TypeInfo::Funcdef { name, .. }) => name.clone(),
        Some(TypeInfo::List(_)) => "list".to_string(),
        None => tt_name(dtype.token).to_string(),
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
    info.as_ref().map(type_info_name).unwrap_or_default()
}

fn type_info_name(info: &TypeInfo) -> String {
    match info {
        TypeInfo::List(_) => "list".into(),
        TypeInfo::Template { name, .. }
        | TypeInfo::Subtype(name)
        | TypeInfo::Object { name, .. }
        | TypeInfo::Funcdef { name, .. } => name.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            asm.lines.iter().map(|line| line.op.as_str()).collect::<Vec<_>>(),
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
            check.detail.as_deref().is_some_and(|text| text.contains("uf[0]")),
            "{:?}",
            check.detail
        );
    }
}
