const std = @import("std");

pub extern fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize) void;
pub extern fn manderrow_agent_deinit(send_exit: bool) void;

extern fn manderrow_agent_report_crash(msg_ptr: [*]const u8, msg_len: usize) void;

/// `msg` must consist solely of UTF-8 characters.
pub fn report_crash(msg: []const u8) !void {
    if (!std.unicode.utf8ValidateSlice(msg)) {
        return error.InvalidMessage;
    }

    manderrow_agent_report_crash(msg.ptr, msg.len);
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

pub const LogLevel = enum(u8) {
    critical,
    err,
    warn,
    info,
    debug,
    trace,
};

/// `scope` must consist solely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist solely of UTF-8 characters.
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
