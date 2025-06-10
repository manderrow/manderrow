const ipc = @import("ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;
const rs = @import("rs.zig");

pub const proxy_exports: [5][:0]const u8 = .{
    "manderrow_agent_init",
    "manderrow_agent_send_crash",
    "manderrow_agent_send_exit",
    "manderrow_agent_send_log",
    "manderrow_agent_send_output_line",
};

pub const InitArgs = extern struct {
    c2s_tx_ptr: ?[*]const u8,
    c2s_tx_len: usize,
    error_buf: *rs.ErrorBuffer,
    /// Return value.
    status: rs.InitStatusCode,
};

pub const SendExitArgs = extern struct { code: i32, with_code: bool };

pub const SendCrashArgs = extern struct { msg_ptr: [*]const u8, msg_len: usize };

pub const SendOutputLineArgs = extern struct {
    channel: ipc.StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
};

pub const SendLogArgs = extern struct {
    level: ipc.LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
};
