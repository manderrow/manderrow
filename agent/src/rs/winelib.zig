const builtin = @import("builtin");
const std = @import("std");

const dlfcn = @import("dlfcn");

const logger = @import("../root.zig").logger;

const crash = @import("../crash.zig").crash;
const ipc = @import("../ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;
const proto = @import("proto.zig");
const rs = @import("../rs.zig");

comptime {
    switch (builtin.cpu.arch) {
        .x86_64 => {},
        else => @compileError("winelib is only supported on x86_64"),
    }
}

pub fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize, error_buf: *rs.ErrorBuffer) rs.InitStatusCode {
    return (init_fn orelse crash(@src(), "not initialized", .{}))(c2s_tx_ptr, c2s_tx_len, error_buf);
}

pub fn manderrow_agent_send_exit(code: i32, with_code: bool) void {
    (send_exit_fn orelse return)(code, with_code);
}

pub fn manderrow_agent_send_crash(msg_ptr: [*]const u8, msg_len: usize) void {
    (send_crash_fn orelse return)(msg_ptr, msg_len);
}

pub fn manderrow_agent_send_output_line(
    channel: StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) void {
    (send_output_line_fn orelse return)(channel, line_ptr, line_len);
}

/// `scope` must consist entirely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist entirely of UTF-8 characters.
pub fn manderrow_agent_send_log(
    level: LogLevel,
    scope_ptr: [*]const u8,
    scope_len: usize,
    msg_ptr: [*]const u8,
    msg_len: usize,
) void {
    (send_log_fn orelse return)(level, scope_ptr, scope_len, msg_ptr, msg_len);
}

comptime {
    if (builtin.os.tag != .windows) {
        @compileError("winelib IPC implementation is only supported on Windows");
    }
}

var init_fn: ?*const proto.init = null;
var send_exit_fn: ?*const proto.send_exit = null;
var send_crash_fn: ?*const proto.send_crash = null;
var send_output_line_fn: ?*const proto.send_output_line = null;
var send_log_fn: ?*const proto.send_log = null;

pub fn init(host_dlfcn_lib_path: [:0]const u16, host_lib_path: [:0]const u8) void {
    logger.debug("Loading host library", .{});

    const dlfcns = dlfcn.init(host_dlfcn_lib_path) catch |e| {
        std.debug.panic("Failed to load host_dlfcn library from {f}: {s}", .{ std.unicode.fmtUtf16Le(host_dlfcn_lib_path), @errorName(e) });
    };

    logger.debug("Loaded host_dlfcn library", .{});

    // TODO: get error string
    const host_lib = dlfcns.dlopen(host_lib_path, .{ .LAZY = true }) orelse {
        std.debug.panic("Failed to load host library from {s}", .{host_lib_path});
    };

    logger.debug("Loaded host library", .{});

    inline for ([5][]const u8{ "init", "send_exit", "send_crash", "send_output_line", "send_log" }) |name| {
        @field(@This(), name ++ "_fn") = @ptrCast(dlfcns.dlsym(host_lib, "manderrow_agent_" ++ name) orelse {
            std.debug.panic("Unable to locate {s} in host library", .{name});
        });

        logger.debug("Located {s}", .{name});
    }
}
