use std::ptr::NonNull;

use crate::{mods::Mod, Error};

pub struct ModIndex {
    data: NonNull<[u8]>,
    mods: Vec<Mod<'static>>,
}

impl ModIndex {
	pub fn new<F, E>(data: Box<[u8]>, mods_constructor: F) -> Result<Self, E>
	where
		F: for<'a> FnOnce(&'a mut [u8]) -> Result<Vec<Mod<'a>>, E>,
	{
		let mut data = NonNull::new(Box::into_raw(data)).unwrap();
		Ok(Self {
			data,
			mods: mods_constructor(unsafe { data.as_mut() })?,
		})
	}
}

impl ModIndex {
    pub fn mods(&self) -> &Vec<Mod<'_>> {
        &self.mods
    }
}

unsafe impl Send for ModIndex {}
unsafe impl Sync for ModIndex {}

impl Drop for ModIndex {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.data.as_ptr()) });
    }
}