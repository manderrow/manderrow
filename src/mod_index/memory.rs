use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};

use rkyv::util::AlignedVec;
use rkyv::vec::ArchivedVec;

use manderrow_types::mods::ArchivedModRef;

/// A game's mod index, in the chunks that Thunderstore serves it in. It can't be changed once it's
/// created, only replaced, so what is worked out from it holds for as long as its id is the same.
pub struct MemoryModIndex {
    id: u64,
    chunks: Vec<MemoryModIndexChunk>,
}

/// Where a mod is in a [`MemoryModIndex`].
#[derive(Clone, Copy)]
pub struct ModPosition {
    chunk: u32,
    index: u32,
}

impl MemoryModIndex {
    pub fn new(chunks: Vec<MemoryModIndexChunk>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            chunks,
        }
    }

    /// Identifies this index among all that have been created.
    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Every mod in the index, with its position.
    pub fn mods(&self) -> impl Iterator<Item = (ModPosition, &ArchivedModRef<'_>)> {
        self.chunks.iter().enumerate().flat_map(|(chunk, mi)| {
            mi.mods().iter().enumerate().map(move |(index, m)| {
                let position = ModPosition {
                    chunk: chunk as u32,
                    index: index as u32,
                };
                (position, m)
            })
        })
    }

    /// The mod at a position in this index.
    pub fn get(&self, position: ModPosition) -> &ArchivedModRef<'_> {
        &self.chunks[position.chunk as usize].mods()[position.index as usize]
    }
}

impl Default for MemoryModIndex {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}

pub struct MemoryModIndexChunk {
    data: NonNull<[u8]>,
    mods: &'static ArchivedVec<ArchivedModRef<'static>>,
}

impl MemoryModIndexChunk {
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

impl MemoryModIndexChunk {
    pub fn mods(&self) -> &ArchivedVec<ArchivedModRef<'_>> {
        // SAFETY: i have a hunch the lifetime issue is a non-issue
        unsafe { NonNull::from(self.mods).cast().as_ref() }
    }
}

unsafe impl Send for MemoryModIndexChunk {}
unsafe impl Sync for MemoryModIndexChunk {}

impl Drop for MemoryModIndexChunk {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.data.as_mut().as_mut_ptr();
            let layout =
                std::alloc::Layout::from_size_align_unchecked(self.data.as_ref().len(), 16);
            std::alloc::dealloc(ptr, layout);
        }
    }
}
