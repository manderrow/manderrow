const builtin = @import("builtin");
const std = @import("std");

const ipc = @import("ipc.zig");
const rs = @import("rs.zig");

// these must be kept in-sync with the definitions in wine_unixlib_proto.zig

pub export fn manderrow_agent_init(
    c2s_tx_ptr: ?[*]const u8,
    c2s_tx_len: usize,
    error_buf: *rs.ErrorBuffer,
) rs.InitStatusCode {
    return rs.impl.manderrow_agent_init(c2s_tx_ptr, c2s_tx_len, error_buf);
}

pub export fn manderrow_agent_send_exit(code: i32, with_code: bool) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_exit(code, with_code);
    return .SUCCESS;
}

pub export fn manderrow_agent_send_crash(msg_ptr: [*]const u8, msg_len: usize) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_crash(msg_ptr, msg_len);
    return .SUCCESS;
}

pub export fn manderrow_agent_send_output_line(
    channel: ipc.StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_output_line(channel, line_ptr, line_len);
    return .SUCCESS;
}

/// `scope` must consist entirely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist entirely of UTF-8 characters.
pub export fn manderrow_agent_send_log(
    level: ipc.LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
) void {
    rs.impl.manderrow_agent_send_log(level, scope_ptr, scope_len, msg_ptr, msg_len);
}
