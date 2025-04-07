use std::ptr::NonNull;

use rkyv::util::AlignedVec;
use rkyv::vec::ArchivedVec;

use manderrow_types::mods::ArchivedModRef;

pub struct MemoryModIndex {
    data: NonNull<[u8]>,
    mods: &'static ArchivedVec<ArchivedModRef<'static>>,
}

impl MemoryModIndex {
    pub fn new<F, E>(mut data: AlignedVec<16>, mods_constructor: F) -> Result<Self, E>
    where
        F: for<'a> FnOnce(&'a [u8]) -> Result<&'a ArchivedVec<ArchivedModRef<'a>>, E>,
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
    pub fn mods(&self) -> &ArchivedVec<ArchivedModRef> {
        // SAFETY: i have a hunch the lifetime issue is a non-issue
        unsafe { NonNull::from(self.mods).cast().as_ref() }
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
