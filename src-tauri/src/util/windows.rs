use std::ffi::c_void;
use std::ptr::NonNull;

#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("Invalid handle")]
pub struct HandleError;

#[repr(transparent)]
pub struct Handle(NonNull<c_void>);

impl Handle {
    pub unsafe fn new(handle: windows::Win32::Foundation::HANDLE) -> Result<Self, HandleError> {
        NonNull::new(handle.0).map(Self).ok_or(HandleError)
    }

    pub fn as_raw(&self) -> windows::Win32::Foundation::HANDLE {
        windows::Win32::Foundation::HANDLE(self.0.as_ptr())
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        unsafe { windows::Win32::Foundation::CloseHandle(self.as_raw()).unwrap() }
    }
}

unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}
