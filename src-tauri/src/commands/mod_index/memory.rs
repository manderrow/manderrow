use std::ptr::NonNull;

use rkyv::util::AlignedVec;
use rkyv::vec::ArchivedVec;

use crate::mods::ArchivedMod;

pub struct MemoryModIndex {
    data: NonNull<[u8]>,
    mods: &'static ArchivedVec<ArchivedMod>,
}

impl MemoryModIndex {
    pub fn new<F, E>(mut data: AlignedVec<16>, mods_constructor: F) -> Result<Self, E>
    where
        F: for<'a> FnOnce(&'a [u8]) -> Result<&'a ArchivedVec<ArchivedMod>, E>,
    {
        data.shrink_to_fit();
        let data_ptr = NonNull::from(data.as_mut_slice());
        std::mem::forget(data);
        Ok(Self {
            data: data_ptr,
            mods: mods_constructor(unsafe { data_ptr.as_ref() })?,
        })
    }
}

impl MemoryModIndex {
    pub fn mods(&self) -> &ArchivedVec<ArchivedMod> {
        &self.mods
    }
}

unsafe impl Send for MemoryModIndex {}
unsafe impl Sync for MemoryModIndex {}

impl Drop for MemoryModIndex {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.as_mut().as_mut_ptr();
            let layout =
                std::alloc::Layout::from_size_align_unchecked(self.data.as_ref().len(), 16);
            std::alloc::dealloc(ptr, layout);
        }
    }
}
