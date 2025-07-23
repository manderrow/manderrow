use std::ptr::NonNull;

use crate::{ErrorBuffer, InitStatusCode};

macro_rules! extern_block {
    ($($tt:tt)*) => {
        #[cfg(target_arch = "x86_64")]
        unsafe extern "sysv64" {
            $($tt)*
        }

        #[cfg(target_arch = "aarch64")]
        unsafe extern "C" {
            $($tt)*
        }
    };
}

macro_rules! extern_fn {
    ($name:ident($($arg_name:ident: $arg_ty:ty),* $(,)?)$( -> $ret_ty:ty)?) => {
        #[cfg(target_arch = "x86_64")]
        #[unsafe(no_mangle)]
        extern "sysv64" fn $name($($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            super::$name($($arg_name),*)
        }

        #[cfg(target_arch = "aarch64")]
        #[unsafe(no_mangle)]
        extern "C" fn $name($($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            super::$name($($arg_name),*)
        }
    };
    (unsafe $name:ident($($arg_name:ident: $arg_ty:ty),* $(,)?)$( -> $ret_ty:ty)?) => {
        #[cfg(target_arch = "x86_64")]
        #[unsafe(no_mangle)]
        unsafe extern "sysv64" fn $name($($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            unsafe { super::$name($($arg_name),*) }
        }

        #[cfg(target_arch = "aarch64")]
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name($($arg_name: $arg_ty),*) $(-> $ret_ty)? {
            unsafe { super::$name($($arg_name),*) }
        }
    };
}

extern_block! {
    pub fn manderrow_agent_crash(msg_ptr: NonNull<u8>, msg_len: usize) -> !;
}

extern_fn!(unsafe manderrow_agent_init(
    c2s_tx_ptr: Option<NonNull<u8>>,
    c2s_tx_len: usize,
    error_buf: &mut ErrorBuffer,
) -> InitStatusCode);

extern_fn!(manderrow_agent_send_exit(code: i32, with_code: bool));

extern_fn!(unsafe manderrow_agent_send_output_line(
    channel: crate::StandardOutputChannel,
    line_ptr: NonNull<u8>,
    line_len: usize,
));

extern_fn!(unsafe manderrow_agent_send_log(
    level: crate::LogLevel,
    scope_ptr: NonNull<u8>,
    scope_len: usize,
    msg_ptr: NonNull<u8>,
    msg_len: usize,
));

extern_fn!(unsafe manderrow_agent_send_crash(msg_ptr: NonNull<u8>, msg_len: usize));
