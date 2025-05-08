const std = @import("std");

const build_options = @import("build_options");
const ipc = @import("ipc.zig");
const LogLevel = ipc.LogLevel;

comptime {
    if (build_options.ipc_mode != .ipc_channel) {
        @compileError("Attempted to access Rust IPC implementation when ipc_mode is not .ipc_channel");
    }
}

pub const ErrorBuffer = extern struct {
    errno: u32,
    message_buf: [*]u8,
    message_len: usize,
};

pub const InitStatusCode = enum(u8) {
    Success,
    ConnectC2SError,
    CreateS2CError,
    SendConnectError,
    RecvConnectError,
    InvalidRecvConnectMessage,
    InvalidPid,
    IpcAlreadySet,
};

pub extern fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize, error_buf: *ErrorBuffer) InitStatusCode;

pub extern fn manderrow_agent_send_exit(code: i32, with_code: bool) void;

extern fn manderrow_agent_send_crash(msg_ptr: [*]const u8, msg_len: usize) void;

/// `msg` must consist entirely of UTF-8 characters.
pub fn sendCrash(msg: []const u8) !void {
    if (!std.unicode.utf8ValidateSlice(msg)) {
        return error.InvalidMessage;
    }

    manderrow_agent_send_crash(msg.ptr, msg.len);
}

pub const StandardOutputChannel = enum(u8) {
    out,
    err,
};

extern fn manderrow_agent_send_output_line(
    channel: StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) void;

/// `line` may consist of arbitrary binary data.
pub fn sendOutputLine(channel: StandardOutputChannel, line: []const u8) void {
    manderrow_agent_send_output_line(channel, line.ptr, line.len);
}

/// `scope` must consist entirely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist entirely of UTF-8 characters.
extern fn manderrow_agent_send_log(
    level: LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
) void;

pub fn sendLog(
    level: LogLevel,
    scope: []const u8,
    msg: []const u8,
) !void {
    for (scope) |c| {
        if (!std.ascii.isPrint(c) or c == ' ') {
            return error.InvalidScope;
        }
    }

    if (!std.unicode.utf8ValidateSlice(msg)) {
        return error.InvalidMessage;
    }

    manderrow_agent_send_log(level, scope.ptr, scope.len, msg.ptr, msg.len);
}
