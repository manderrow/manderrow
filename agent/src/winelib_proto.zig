const std = @import("std");

const ipc = @import("ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;
const rs = @import("rs.zig");

pub const calling_convention: std.builtin.CallingConvention = .{ .x86_64_sysv = .{} };

pub const init = fn (
    c2s_tx_ptr: ?[*]const u8,
    c2s_tx_len: usize,
    error_buf: *rs.ErrorBuffer,
) callconv(calling_convention) rs.InitStatusCode;

pub const send_exit = fn (code: i32, with_code: bool) callconv(calling_convention) void;

pub const send_crash = fn (msg_ptr: [*]const u8, msg_len: usize) callconv(calling_convention) void;

pub const send_output_line = fn (
    channel: ipc.StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) callconv(calling_convention) void;

pub const send_log = fn (
    level: ipc.LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
) callconv(calling_convention) void;
