// This is a generated file! Please edit source .ksy file and use kaitai-struct-compiler to rebuild

#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(irrefutable_let_patterns)]
#![allow(unused_comparisons)]

extern crate kaitai;
use kaitai::*;
use std::cell::{Cell, Ref, RefCell};
use std::convert::{TryFrom, TryInto};
use std::rc::{Rc, Weak};

#[derive(Default, Debug, Clone)]
pub struct Fms {
    pub _root: SharedType<Fms>,
    pub _parent: SharedType<Fms>,
    pub _self: SharedType<Self>,
    magic: RefCell<Vec<u8>>,
    payload_size: RefCell<u32>,
    header_size: RefCell<u32>,
    reserved_0c: RefCell<Vec<u8>>,
    string_count: RefCell<u32>,
    encoding: RefCell<u8>,
    reserved_19: RefCell<Vec<u8>>,
    slots: RefCell<Vec<u8>>,
    strings: RefCell<Vec<OptRc<Fms_Text>>>,
    tail: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Fms {
    type Root = Fms;
    type Parent = Fms;

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
        if !(*self_rc.magic() == vec![0x46u8, 0x4du8, 0x53u8, 0x42u8]) {
            return Err(KError::ValidationFailed(ValidationFailedError {
                kind: ValidationKind::NotEqual,
                src_path: "/seq/0".to_string(),
            }));
        }
        *self_rc.payload_size.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.header_size.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.reserved_0c.borrow_mut() = _io.read_bytes(8 as usize)?.into();
        *self_rc.string_count.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.encoding.borrow_mut() = _io.read_u1()?.into();
        *self_rc.reserved_19.borrow_mut() = _io
            .read_bytes(((*self_rc.header_size() as u32) - (25 as u32)) as usize)?
            .into();
        *self_rc.slots.borrow_mut() = _io
            .read_bytes(((*self_rc.string_count() as u32) * (8 as u32)) as usize)?
            .into();
        *self_rc.strings.borrow_mut() = Vec::new();
        let l_strings = *self_rc.string_count();
        for _i in 0..l_strings {
            let t = Self::read_into::<_, Fms_Text>(
                &*_io,
                Some(self_rc._root.clone()),
                Some(self_rc._self.clone()),
            )?
            .into();
            self_rc.strings.borrow_mut().push(t);
        }
        *self_rc.tail.borrow_mut() = _io.read_bytes_full()?.into();
        Ok(())
    }
}
impl Fms {}
impl Fms {
    pub fn magic(&self) -> Ref<'_, Vec<u8>> {
        self.magic.borrow()
    }
}
impl Fms {
    pub fn payload_size(&self) -> Ref<'_, u32> {
        self.payload_size.borrow()
    }
}
impl Fms {
    pub fn header_size(&self) -> Ref<'_, u32> {
        self.header_size.borrow()
    }
}
impl Fms {
    pub fn reserved_0c(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_0c.borrow()
    }
}
impl Fms {
    pub fn string_count(&self) -> Ref<'_, u32> {
        self.string_count.borrow()
    }
}
impl Fms {
    pub fn encoding(&self) -> Ref<'_, u8> {
        self.encoding.borrow()
    }
}
impl Fms {
    pub fn reserved_19(&self) -> Ref<'_, Vec<u8>> {
        self.reserved_19.borrow()
    }
}
impl Fms {
    pub fn slots(&self) -> Ref<'_, Vec<u8>> {
        self.slots.borrow()
    }
}
impl Fms {
    pub fn strings(&self) -> Ref<'_, Vec<OptRc<Fms_Text>>> {
        self.strings.borrow()
    }
}
impl Fms {
    pub fn tail(&self) -> Ref<'_, Vec<u8>> {
        self.tail.borrow()
    }
}
impl Fms {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Fms_Text {
    pub _root: SharedType<Fms>,
    pub _parent: SharedType<Fms>,
    pub _self: SharedType<Self>,
    value: RefCell<String>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Fms_Text {
    type Root = Fms;
    type Parent = Fms;

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
        *self_rc.value.borrow_mut() =
            bytes_to_str(&_io.read_bytes_term(0, false, true, true)?.into(), "UTF-8")?;
        Ok(())
    }
}
impl Fms_Text {}
impl Fms_Text {
    pub fn value(&self) -> Ref<'_, String> {
        self.value.borrow()
    }
}
impl Fms_Text {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
