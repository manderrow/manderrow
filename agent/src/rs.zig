const std = @import("std");

const build_options = @import("build_options");
const ipc = @import("ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;

pub const impl = switch (build_options.ipc_mode) {
    .ipc_channel => @import("rs/ipc_channel.zig"),
    .wine_unixlib => @import("rs/wine_unixlib.zig"),
    .stderr => @compileError("cannot use rs.zig when ipc_mode is stderr"),
};

comptime {
    switch (build_options.ipc_mode) {
        .ipc_channel, .wine_unixlib => {},
        .stderr => @compileError("Attempted to access Rust IPC implementation when ipc_mode is not .ipc_channel or .wine_unixlib"),
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

pub fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize, error_buf: *ErrorBuffer) InitStatusCode {
    return impl.manderrow_agent_init(c2s_tx_ptr, c2s_tx_len, error_buf);
}

pub fn manderrow_agent_send_exit(code: i32, with_code: bool) void {
    impl.manderrow_agent_send_exit(code, with_code);
}

/// `msg` must consist entirely of UTF-8 characters.
pub fn sendCrash(msg: []const u8) !void {
    if (!std.unicode.utf8ValidateSlice(msg)) {
        return error.InvalidMessage;
    }

    impl.manderrow_agent_send_crash(msg.ptr, msg.len);
}

/// `line` may consist of arbitrary binary data.
pub fn sendOutputLine(channel: StandardOutputChannel, line: []const u8) void {
    impl.manderrow_agent_send_output_line(channel, line.ptr, line.len);
}

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

    impl.manderrow_agent_send_log(level, scope.ptr, scope.len, msg.ptr, msg.len);
}
