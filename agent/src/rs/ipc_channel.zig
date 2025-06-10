const builtin = @import("builtin");
const std = @import("std");

const ipc = @import("../ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;
const rs = @import("../rs.zig");

pub extern fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize, error_buf: *rs.ErrorBuffer) rs.InitStatusCode;

pub extern fn manderrow_agent_send_exit(code: i32, with_code: bool) void;

pub extern fn manderrow_agent_send_crash(msg_ptr: [*]const u8, msg_len: usize) void;

pub extern fn manderrow_agent_send_output_line(
    channel: StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) void;

/// `scope` must consist entirely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist entirely of UTF-8 characters.
pub extern fn manderrow_agent_send_log(
    level: LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
) void;
