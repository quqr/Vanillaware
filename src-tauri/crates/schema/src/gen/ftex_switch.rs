// This is a generated file! Please edit source .ksy file and use kaitai-struct-compiler to rebuild

#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(irrefutable_let_patterns)]
#![allow(unused_comparisons)]

extern crate kaitai;
use kaitai::*;
use std::convert::{TryFrom, TryInto};
use std::cell::{Ref, Cell, RefCell};
use std::rc::{Rc, Weak};

/**
 * 结构来源：VanillawareConverter/FTEXConverter/SwitchFtexParser.cs（逐字段等价）。
 * 真实样本：F:\Codes\13\13rim ex\EX DATA\BattleEffect\Barrier00.ftx（262608 B）
 *   magic "FTEX" @0x00, version 0x00040140 @0x04, header_size 0x80 @0x08, texture_count 1 @0x0C
 *   name @0x20 (0x20 B + 0x10 B 保留) = 0x30 B/条
 *   FTX0 @0x80: nvt_size 0x00040100 @+0x04, nvt_offset 0x40 @+0x08
 *   NVT  @0x80+0x40: ".tex", format 0x4D(BC7) @+0x04, w 512 @+0x0C, h 512 @+0x10,
 *                    data_offset 0x100 @+0x1C, data_size 0x40000 @+0x20
 * 
 * 注意：本文件只负责「结构」。像素解码（BC3/BC4/BC7、TegraX1 swizzle、FCMP LZSS）
 * 在 formats 的 codec 层手写，Kaitai 不承担。
 */

#[derive(Default, Debug, Clone)]
pub struct FtexSwitch {
    pub _root: SharedType<FtexSwitch>,
    pub _parent: SharedType<FtexSwitch>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    version: RefCell<u32>,
    header_size: RefCell<u32>,
    texture_count: RefCell<u32>,
    reserved_10: RefCell<Vec<u8>>,
    entries: RefCell<Vec<OptRc<FtexSwitch_TextureEntry>>>,
    header_padding: RefCell<Vec<u8>>,
    blocks: RefCell<Vec<OptRc<FtexSwitch_FtxBlock>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for FtexSwitch {
    type Root = FtexSwitch;
    type Parent = FtexSwitch;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.magic.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        if !(*self_rc.magic() == vec![0x46u8, 0x54u8, 0x45u8, 0x58u8]) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::NotEqual, src_path: "/seq/0".to_string() }));
        }
        *self_rc.version.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.header_size.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.texture_count.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_10.borrow_mut() = _io.read_bytes(16 as usize)?.into();
        *self_rc.entries.borrow_mut() = Vec::new();
        let l_entries = *self_rc.texture_count();
        for _i in 0..l_entries {
            let t = Self::read_into::<_, FtexSwitch_TextureEntry>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.entries.borrow_mut().push(t);
        }
        if ((*self_rc.header_size() as i32) > (((32 as i32) + (((*self_rc.texture_count() as u32) * (48 as u32)) as i32)) as i32)) {
            *self_rc.header_padding.borrow_mut() = _io.read_bytes(((*self_rc.header_size() as i32) - (((32 as i32) + (((*self_rc.texture_count() as u32) * (48 as u32)) as i32)) as i32)) as usize)?.into();
        }
        *self_rc.blocks.borrow_mut() = Vec::new();
        let l_blocks = *self_rc.texture_count();
        for _i in 0..l_blocks {
            let t = Self::read_into::<_, FtexSwitch_FtxBlock>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.blocks.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl FtexSwitch {
}
impl FtexSwitch {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}

/**
 * 0x00040140 等，含义未解，仅透传
 */
impl FtexSwitch {
    pub fn version(&self) -> Ref<'_, u32> {
        self.version.borrow()
    }
}

/**
 * 首个 FTX0 块相对文件起点的偏移
 */
impl FtexSwitch {
    pub fn header_size(&self) -> Ref<'_, u32> {
        self.header_size.borrow()
    }
}
impl FtexSwitch {
    pub fn texture_count(&self) -> Ref<'_, u32> {
        self.texture_count.borrow()
    }
}

/**
 * 0x10..0x20
 */
impl FtexSwitch {
    pub fn reserved_10(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_10.borrow()
    }
}
impl FtexSwitch {
    pub fn entries(&self) -> Ref<'_, Vec<OptRc<FtexSwitch_TextureEntry>>> {
        self.entries.borrow()
    }
}

/**
 * 名字表之后到首个块之间的填充
 */
impl FtexSwitch {
    pub fn header_padding(&self) -> Ref<'_, Vec<u8>> {
        self.header_padding.borrow()
    }
}

/**
 * 块按 sz1+sz2 顺序紧挨排列，可顺序读取
 */
impl FtexSwitch {
    pub fn blocks(&self) -> Ref<'_, Vec<OptRc<FtexSwitch_FtxBlock>>> {
        self.blocks.borrow()
    }
}
impl FtexSwitch {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct FtexSwitch_FtxBlock {
    pub _root: SharedType<FtexSwitch>,
    pub _parent: SharedType<FtexSwitch>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    nvt_size: RefCell<u32>,
    nvt_offset: RefCell<u32>,
    padding: RefCell<Vec<u8>>,
    nvt: RefCell<OptRc<FtexSwitch_Nvt>>,
    _io: RefCell<BytesReader>,
    nvt_raw: RefCell<Vec<u8>>,
}
impl KStruct for FtexSwitch_FtxBlock {
    type Root = FtexSwitch;
    type Parent = FtexSwitch;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.magic.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        if !(*self_rc.magic() == vec![0x46u8, 0x54u8, 0x58u8, 0x30u8]) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::NotEqual, src_path: "/types/ftx_block/seq/0".to_string() }));
        }
        *self_rc.nvt_size.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.nvt_offset.borrow_mut() = _io.read_u4le()?.into();
        if ((*self_rc.nvt_offset() as u32) > (12 as u32)) {
            *self_rc.padding.borrow_mut() = _io.read_bytes(((*self_rc.nvt_offset() as u32) - (12 as u32)) as usize)?.into();
        }
        *self_rc.nvt_raw.borrow_mut() = _io.read_bytes(*self_rc.nvt_size() as usize)?.into();
        let nvt_raw = self_rc.nvt_raw.borrow();
        let _t_nvt_raw_io = BytesReader::from(nvt_raw.clone());
        let t = Self::read_into::<BytesReader, FtexSwitch_Nvt>(&_t_nvt_raw_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
        *self_rc.nvt.borrow_mut() = t;
        Ok(())
    }
}
impl FtexSwitch_FtxBlock {
}
impl FtexSwitch_FtxBlock {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}

/**
 * NVT 数据长度（样本 0x00040100）
 */
impl FtexSwitch_FtxBlock {
    pub fn nvt_size(&self) -> Ref<'_, u32> {
        self.nvt_size.borrow()
    }
}

/**
 * NVT 相对块起点的偏移（样本 0x40）
 */
impl FtexSwitch_FtxBlock {
    pub fn nvt_offset(&self) -> Ref<'_, u32> {
        self.nvt_offset.borrow()
    }
}
impl FtexSwitch_FtxBlock {
    pub fn padding(&self) -> Ref<'_, Vec<u8>> {
        self.padding.borrow()
    }
}
impl FtexSwitch_FtxBlock {
    pub fn nvt(&self) -> Ref<'_, OptRc<FtexSwitch_Nvt>> {
        self.nvt.borrow()
    }
}
impl FtexSwitch_FtxBlock {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
impl FtexSwitch_FtxBlock {
    pub fn nvt_raw(&self) -> Ref<'_, Vec<u8>> {
        self.nvt_raw.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct FtexSwitch_Nvt {
    pub _root: SharedType<FtexSwitch>,
    pub _parent: SharedType<FtexSwitch_FtxBlock>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    format: RefCell<u16>,
    reserved_06: RefCell<Vec<u8>>,
    flags: RefCell<u32>,
    width: RefCell<u32>,
    height: RefCell<u32>,
    reserved_14: RefCell<Vec<u8>>,
    data_offset: RefCell<u32>,
    data_size: RefCell<u32>,
    data_padding: RefCell<Vec<u8>>,
    data: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for FtexSwitch_Nvt {
    type Root = FtexSwitch;
    type Parent = FtexSwitch_FtxBlock;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.magic.borrow_mut() = _io.read_bytes(4 as usize)?.into();
        if !(*self_rc.magic() == vec![0x2eu8, 0x74u8, 0x65u8, 0x78u8]) {
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::NotEqual, src_path: "/types/nvt/seq/0".to_string() }));
        }
        *self_rc.format.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.reserved_06.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.width.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.height.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_14.borrow_mut() = _io.read_bytes(8 as usize)?.into();
        *self_rc.data_offset.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.data_size.borrow_mut() = _io.read_u4le()?.into();
        if ((*self_rc.data_offset() as u32) > (36 as u32)) {
            *self_rc.data_padding.borrow_mut() = _io.read_bytes(((*self_rc.data_offset() as u32) - (36 as u32)) as usize)?.into();
        }
        *self_rc.data.borrow_mut() = _io.read_bytes(*self_rc.data_size() as usize)?.into();
        Ok(())
    }
}
impl FtexSwitch_Nvt {
}
impl FtexSwitch_Nvt {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}

/**
 * 0x44 BC3 / 0x49 BC4 / 0x4D BC7。故意不用 enum：未列出的值会让生成的代码在 try_into() 处报 NotInEnum
 */
impl FtexSwitch_Nvt {
    pub fn format(&self) -> Ref<'_, u16> {
        self.format.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn reserved_06(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_06.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn width(&self) -> Ref<'_, u32> {
        self.width.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn height(&self) -> Ref<'_, u32> {
        self.height.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn reserved_14(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_14.borrow()
    }
}

/**
 * 像素数据相对 .tex 起点的偏移（样本 0x100）
 */
impl FtexSwitch_Nvt {
    pub fn data_offset(&self) -> Ref<'_, u32> {
        self.data_offset.borrow()
    }
}

/**
 * 像素数据长度（样本 0x40000）
 */
impl FtexSwitch_Nvt {
    pub fn data_size(&self) -> Ref<'_, u32> {
        self.data_size.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn data_padding(&self) -> Ref<'_, Vec<u8>> {
        self.data_padding.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn data(&self) -> Ref<'_, Vec<u8>> {
        self.data.borrow()
    }
}
impl FtexSwitch_Nvt {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct FtexSwitch_TextureEntry {
    pub _root: SharedType<FtexSwitch>,
    pub _parent: SharedType<FtexSwitch>,
    pub _self: SharedType<Self>,
    name: RefCell<Vec<u8>>,
    reserved: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for FtexSwitch_TextureEntry {
    type Root = FtexSwitch;
    type Parent = FtexSwitch;

    fn read<S: KStream>(
        self_rc: &OptRc<Self>,
        _io: &S,
        _root: SharedType<Self::Root>,
        _parent: SharedType<Self::Parent>,
    ) -> KResult<()> {
        *self_rc._io.borrow_mut() = _io.clone();
        self_rc._root.set(_root.get());
        self_rc._parent.set(_parent.get());
        self_rc._self.set(Ok(self_rc.clone()));
        let _rrc = self_rc._root.get_value().borrow().upgrade();
        let _prc = self_rc._parent.get_value().borrow().upgrade();
        let _r = _rrc.as_ref().unwrap();
        *self_rc.name.borrow_mut() = _io.read_bytes(32 as usize)?.into();
        *self_rc.reserved.borrow_mut() = _io.read_bytes(16 as usize)?.into();
        Ok(())
    }
}
impl FtexSwitch_TextureEntry {
}

/**
 * NUL 结尾的 ASCII 文件名，Rust 侧用 from_utf8_lossy 解码
 */
impl FtexSwitch_TextureEntry {
    pub fn name(&self) -> Ref<'_, Vec<u8>> {
        self.name.borrow()
    }
}
impl FtexSwitch_TextureEntry {
    pub fn reserved(&self) -> Ref<'_, Vec<u8>> {
        self.reserved.borrow()
    }
}
impl FtexSwitch_TextureEntry {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
