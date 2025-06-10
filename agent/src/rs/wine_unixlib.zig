const builtin = @import("builtin");
const std = @import("std");

const logger = @import("../root.zig").logger;

const ipc = @import("../ipc.zig");
const LogLevel = ipc.LogLevel;
const StandardOutputChannel = ipc.StandardOutputChannel;
const proto = @import("../wine_unixlib_proto.zig");
const rs = @import("../rs.zig");

pub fn manderrow_agent_init(c2s_tx_ptr: ?[*]const u8, c2s_tx_len: usize, error_buf: *rs.ErrorBuffer) rs.InitStatusCode {
    var args: proto.InitArgs = .{
        .c2s_tx_ptr = c2s_tx_ptr,
        .c2s_tx_len = c2s_tx_len,
        .error_buf = error_buf,
        .status = undefined,
    };
    wine_unix_call(.manderrow_agent_init, &args);
    return args.status;
}

pub fn manderrow_agent_send_exit(code: i32, with_code: bool) void {
    var args: proto.SendExitArgs = .{
        .code = code,
        .with_code = with_code,
    };
    wine_unix_call(.manderrow_agent_send_exit, &args);
}

pub fn manderrow_agent_send_crash(msg_ptr: [*]const u8, msg_len: usize) void {
    var args: proto.SendCrashArgs = .{
        .msg_ptr = msg_ptr,
        .msg_len = msg_len,
    };
    wine_unix_call(.manderrow_agent_send_crash, &args);
}

pub fn manderrow_agent_send_output_line(
    channel: StandardOutputChannel,
    line_ptr: [*]const u8,
    line_len: usize,
) void {
    var args: proto.SendOutputLineArgs = .{
        .channel = channel,
        .line_ptr = line_ptr,
        .line_len = line_len,
    };
    wine_unix_call(.manderrow_agent_send_output_line, &args);
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
    var args: proto.SendLogArgs = .{
        .level = level,
        .scope_ptr = scope_ptr,
        .scope_len = scope_len,
        .msg_ptr = msg_ptr,
        .msg_len = msg_len,
    };
    wine_unix_call(.manderrow_agent_send_log, &args);
}

comptime {
    if (builtin.os.tag != .windows) {
        @compileError("wine_unixlib IPC implementation is only supported on Windows");
    }
}

const wine_unix_call_code = blk: {
    var fields: []const std.builtin.Type.EnumField = &.{};
    for (proto.proxy_exports, 0..) |name, i| {
        fields = fields ++ .{std.builtin.Type.EnumField{
            .name = name,
            .value = i,
        }};
    }
    break :blk @Type(.{
        .@"enum" = .{
            .decls = &.{},
            .is_exhaustive = true,
            .tag_type = c_uint,
            .fields = fields,
        },
    });
};

const unixlib_handle_t = u64;

var unixlib: unixlib_handle_t = undefined;

pub fn init(host_lib_path: [:0]const u16) void {
    logger.debug("Loading host library", .{});

    const module = std.os.windows.LoadLibraryW(host_lib_path) catch |e| {
        std.debug.panic("Failed to load host library from {}: {s}", .{ std.unicode.fmtUtf16Le(host_lib_path), @errorName(e) });
    };

    logger.debug("Loaded host library", .{});

    const status = NtQueryVirtualMemory(std.os.windows.GetCurrentProcess(), module, .MemoryWineUnixFuncs, &unixlib, @sizeOf(unixlib_handle_t), null);
    if (status != .SUCCESS) {
        std.debug.panic("Failed to query host library functions: 0x{x}", .{@intFromEnum(status)});
    }

    logger.debug("Queried host library functions", .{});

    const ntdll = std.os.windows.kernel32.GetModuleHandleW(std.unicode.utf8ToUtf16LeStringLiteral("ntdll.dll")) orelse @panic("Unable to find ntdll");
    __wine_unix_call = @ptrCast(std.os.windows.kernel32.GetProcAddress(ntdll, "__wine_unix_call") orelse @panic("Unable to locate __wine_unix_call in ntdll"));

    logger.debug("Located __wine_unix_call", .{});
}

fn wine_unix_call(code: wine_unix_call_code, args: *anyopaque) void {
    const status = (__wine_unix_call orelse return)(unixlib, @intFromEnum(code), args);
    if (status != .SUCCESS) {
        std.debug.panic("Failed to call host library function: 0x{x}", .{@intFromEnum(status)});
    }
}

const MEMORY_INFORMATION_CLASS = enum(c_int) {
    MemoryBasicInformation,
    MemoryWorkingSetInformation,
    MemoryMappedFilenameInformation,
    MemoryRegionInformation,
    MemoryWorkingSetExInformation,
    MemorySharedCommitInformation,
    MemoryImageInformation,
    MemoryRegionInformationEx,
    MemoryPrivilegedBasicInformation,
    MemoryEnclaveImageInformation,
    MemoryBasicInformationCapped,
    MemoryPhysicalContiguityInformation,
    MemoryBadInformation,
    MemoryBadInformationAllProcesses,
    MemoryWineUnixFuncs = 1000,
    MemoryWineUnixWow64Funcs,
    _,
};

extern "ntdll" fn NtQueryVirtualMemory(
    ProcessHandle: std.os.windows.HANDLE,
    BaseAddress: ?std.os.windows.PVOID,
    MemoryInformationClass: MEMORY_INFORMATION_CLASS,
    MemoryInformation: std.os.windows.PVOID,
    MemoryInformationLength: std.os.windows.SIZE_T,
    ReturnLength: ?*std.os.windows.SIZE_T,
) callconv(.winapi) std.os.windows.NTSTATUS;

const WineUnixCall = fn (
    handle: unixlib_handle_t,
    code: c_uint,
    args: *anyopaque,
) callconv(.winapi) std.os.windows.NTSTATUS;

var __wine_unix_call: ?*const WineUnixCall = null;
