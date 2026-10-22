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
 * 与 vw_ftx.ksy 结构相同，但 **不读取像素数据**：NVT 只解析到 data_offset/data_size 就停。
 * 用途：帧数/尺寸/格式补全（probe）。整块像素由 Kaitai 读进 Vec<u8> 的话，一个 83MB 的
 * ftx 只想知道"有几帧"也要分配 83MB，所以这里刻意留空，解码路径仍走 vw_ftx.ksy。
 */

#[derive(Default, Debug, Clone)]
pub struct VwFtxIndex {
    pub _root: SharedType<VwFtxIndex>,
    pub _parent: SharedType<VwFtxIndex>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    version: RefCell<u32>,
    header_size: RefCell<u32>,
    texture_count: RefCell<u32>,
    reserved_10: RefCell<Vec<u8>>,
    entries: RefCell<Vec<OptRc<VwFtxIndex_TextureEntry>>>,
    header_padding: RefCell<Vec<u8>>,
    blocks: RefCell<Vec<OptRc<VwFtxIndex_FtxBlock>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for VwFtxIndex {
    type Root = VwFtxIndex;
    type Parent = VwFtxIndex;

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
            let t = Self::read_into::<_, VwFtxIndex_TextureEntry>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.entries.borrow_mut().push(t);
        }
        if ((*self_rc.header_size() as i32) > (((32 as i32) + (((*self_rc.texture_count() as u32) * (48 as u32)) as i32)) as i32)) {
            *self_rc.header_padding.borrow_mut() = _io.read_bytes(((*self_rc.header_size() as i32) - (((32 as i32) + (((*self_rc.texture_count() as u32) * (48 as u32)) as i32)) as i32)) as usize)?.into();
        }
        *self_rc.blocks.borrow_mut() = Vec::new();
        let l_blocks = *self_rc.texture_count();
        for _i in 0..l_blocks {
            let t = Self::read_into::<_, VwFtxIndex_FtxBlock>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.blocks.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl VwFtxIndex {
}
impl VwFtxIndex {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}
impl VwFtxIndex {
    pub fn version(&self) -> Ref<'_, u32> {
        self.version.borrow()
    }
}
impl VwFtxIndex {
    pub fn header_size(&self) -> Ref<'_, u32> {
        self.header_size.borrow()
    }
}
impl VwFtxIndex {
    pub fn texture_count(&self) -> Ref<'_, u32> {
        self.texture_count.borrow()
    }
}
impl VwFtxIndex {
    pub fn reserved_10(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_10.borrow()
    }
}
impl VwFtxIndex {
    pub fn entries(&self) -> Ref<'_, Vec<OptRc<VwFtxIndex_TextureEntry>>> {
        self.entries.borrow()
    }
}
impl VwFtxIndex {
    pub fn header_padding(&self) -> Ref<'_, Vec<u8>> {
        self.header_padding.borrow()
    }
}
impl VwFtxIndex {
    pub fn blocks(&self) -> Ref<'_, Vec<OptRc<VwFtxIndex_FtxBlock>>> {
        self.blocks.borrow()
    }
}
impl VwFtxIndex {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct VwFtxIndex_FtxBlock {
    pub _root: SharedType<VwFtxIndex>,
    pub _parent: SharedType<VwFtxIndex>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    nvt_size: RefCell<u32>,
    nvt_offset: RefCell<u32>,
    padding: RefCell<Vec<u8>>,
    nvt: RefCell<OptRc<VwFtxIndex_NvtHeader>>,
    _io: RefCell<BytesReader>,
    nvt_raw: RefCell<Vec<u8>>,
}
impl KStruct for VwFtxIndex_FtxBlock {
    type Root = VwFtxIndex;
    type Parent = VwFtxIndex;

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
        let t = Self::read_into::<BytesReader, VwFtxIndex_NvtHeader>(&_t_nvt_raw_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
        *self_rc.nvt.borrow_mut() = t;
        Ok(())
    }
}
impl VwFtxIndex_FtxBlock {
}
impl VwFtxIndex_FtxBlock {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn nvt_size(&self) -> Ref<'_, u32> {
        self.nvt_size.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn nvt_offset(&self) -> Ref<'_, u32> {
        self.nvt_offset.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn padding(&self) -> Ref<'_, Vec<u8>> {
        self.padding.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn nvt(&self) -> Ref<'_, OptRc<VwFtxIndex_NvtHeader>> {
        self.nvt.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
impl VwFtxIndex_FtxBlock {
    pub fn nvt_raw(&self) -> Ref<'_, Vec<u8>> {
        self.nvt_raw.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct VwFtxIndex_NvtHeader {
    pub _root: SharedType<VwFtxIndex>,
    pub _parent: SharedType<VwFtxIndex_FtxBlock>,
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
    _io: RefCell<BytesReader>,
}
impl KStruct for VwFtxIndex_NvtHeader {
    type Root = VwFtxIndex;
    type Parent = VwFtxIndex_FtxBlock;

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
            return Err(KError::ValidationFailed(ValidationFailedError { kind: ValidationKind::NotEqual, src_path: "/types/nvt_header/seq/0".to_string() }));
        }
        *self_rc.format.borrow_mut() = _io.read_u2le()?.into();
        *self_rc.reserved_06.borrow_mut() = _io.read_bytes(2 as usize)?.into();
        *self_rc.flags.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.width.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.height.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_14.borrow_mut() = _io.read_bytes(8 as usize)?.into();
        *self_rc.data_offset.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.data_size.borrow_mut() = _io.read_u4le()?.into();
        Ok(())
    }
}
impl VwFtxIndex_NvtHeader {
}
impl VwFtxIndex_NvtHeader {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn format(&self) -> Ref<'_, u16> {
        self.format.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn reserved_06(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_06.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn flags(&self) -> Ref<'_, u32> {
        self.flags.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn width(&self) -> Ref<'_, u32> {
        self.width.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn height(&self) -> Ref<'_, u32> {
        self.height.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn reserved_14(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_14.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn data_offset(&self) -> Ref<'_, u32> {
        self.data_offset.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn data_size(&self) -> Ref<'_, u32> {
        self.data_size.borrow()
    }
}
impl VwFtxIndex_NvtHeader {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct VwFtxIndex_TextureEntry {
    pub _root: SharedType<VwFtxIndex>,
    pub _parent: SharedType<VwFtxIndex>,
    pub _self: SharedType<Self>,
    name: RefCell<Vec<u8>>,
    reserved: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for VwFtxIndex_TextureEntry {
    type Root = VwFtxIndex;
    type Parent = VwFtxIndex;

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
impl VwFtxIndex_TextureEntry {
}
impl VwFtxIndex_TextureEntry {
    pub fn name(&self) -> Ref<'_, Vec<u8>> {
        self.name.borrow()
    }
}
impl VwFtxIndex_TextureEntry {
    pub fn reserved(&self) -> Ref<'_, Vec<u8>> {
        self.reserved.borrow()
    }
}
impl VwFtxIndex_TextureEntry {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
