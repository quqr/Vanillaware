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

#[derive(Default, Debug, Clone)]
pub struct Scb {
    pub _root: SharedType<Scb>,
    pub _parent: SharedType<Scb>,
    pub _self: SharedType<Self>,
    token_count: RefCell<u32>,
    tokens: RefCell<Vec<OptRc<Scb_Token>>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Scb {
    type Root = Scb;
    type Parent = Scb;

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
        *self_rc.token_count.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.tokens.borrow_mut() = Vec::new();
        let l_tokens = *self_rc.token_count();
        for _i in 0..l_tokens {
            let t = Self::read_into::<_, Scb_Token>(&*_io, Some(self_rc._root.clone()), Some(self_rc._self.clone()))?.into();
            self_rc.tokens.borrow_mut().push(t);
        }
        Ok(())
    }
}
impl Scb {
}
impl Scb {
    pub fn token_count(&self) -> Ref<'_, u32> {
        self.token_count.borrow()
    }
}
impl Scb {
    pub fn tokens(&self) -> Ref<'_, Vec<OptRc<Scb_Token>>> {
        self.tokens.borrow()
    }
}
impl Scb {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}

#[derive(Default, Debug, Clone)]
pub struct Scb_Token {
    pub _root: SharedType<Scb>,
    pub _parent: SharedType<Scb>,
    pub _self: SharedType<Self>,
    len: RefCell<u32>,
    raw: RefCell<Vec<u8>>,
    _io: RefCell<BytesReader>,
}
impl KStruct for Scb_Token {
    type Root = Scb;
    type Parent = Scb;

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
        *self_rc.len.borrow_mut() = _io.read_u4le()?.into();
        *self_rc.raw.borrow_mut() = _io.read_bytes(*self_rc.len() as usize)?.into();
        Ok(())
    }
}
impl Scb_Token {
}
impl Scb_Token {
    pub fn len(&self) -> Ref<'_, u32> {
        self.len.borrow()
    }
}
impl Scb_Token {
    pub fn raw(&self) -> Ref<'_, Vec<u8>> {
        self.raw.borrow()
    }
}
impl Scb_Token {
    pub fn _io(&self) -> Ref<'_, BytesReader> {
        self._io.borrow()
    }
}
