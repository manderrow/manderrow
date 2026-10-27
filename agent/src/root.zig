const builtin = @import("builtin");
const std = @import("std");
const dll_proxy = @import("dll_proxy");

const build_options = @import("build_options");
const Args = @import("Args.zig");
const crash = @import("crash.zig");
const ipc = @import("ipc.zig");
const paths = @import("paths.zig");
const rs = @import("rs.zig");
const stdio = @import("stdio.zig");
const util = @import("util.zig");
const windows = @import("windows.zig");

pub const std_options: std.Options = .{
    .log_level = .debug,
    .logFn = logFn,
    .enable_segfault_handler = false,
};

pub const panic = std.debug.FullPanic(crash.panic);

// rather than Io.Threaded.init, which installs signal handlers in the process that loaded the agent
var threaded: std.Io.Threaded = .init_single_threaded;
pub const io = threaded.io();

/// The process's arguments, which std only gives an executable's main.
var argv: if (builtin.target.os.tag == .windows) void else []const [*:0]const u8 = if (builtin.target.os.tag == .windows) {} else &.{};

pub fn processArgs() std.process.Args {
    return .{ .vector = switch (builtin.target.os.tag) {
        .windows => std.os.windows.peb().ProcessParameters.CommandLine.slice(),
        else => argv,
    } };
}

/// The process's environment as it is now, which std only gives an executable's main.
pub fn processEnviron() std.process.Environ {
    return .{
        .block = switch (builtin.target.os.tag) {
            .windows => .global,
            // a shared library can't link to environ on macOS
            .macos => .{ .slice = std.mem.span(_NSGetEnviron().*) },
            else => .{ .slice = std.mem.span(std.c.environ) },
        },
    };
}

var log_file: ?std.Io.File = null;
var log_file_lock: util.RecursiveMutex = .init;

fn logFn(
    comptime message_level: std.log.Level,
    comptime scope: @TypeOf(.enum_literal),
    comptime format: []const u8,
    args: anytype,
) void {
    const level: ipc.LogLevel = switch (message_level) {
        .debug => .debug,
        .info => .info,
        .warn => .warn,
        .err => .err,
    };

    logToLogFile(level, @tagName(scope), format, args);

    switch (build_options.ipc_mode) {
        .ipc_channel, .winelib => {
            const buf = std.fmt.allocPrint(alloc, format, args) catch return;
            defer alloc.free(buf);
            rs.sendLog(level, @tagName(scope), buf) catch return;
        },
        .stderr => {
            var buf: [4096]u8 = undefined;
            const stderr = std.debug.lockStderr(&buf);
            defer std.debug.unlockStderr();
            writeLog(&stderr.file_writer.interface, level, @tagName(scope), format, args);
        },
    }
}

pub fn logToLogFile(
    message_level: ipc.LogLevel,
    scope: []const u8,
    comptime format: []const u8,
    args: anytype,
) void {
    if (log_file) |f| {
        log_file_lock.lock();
        defer log_file_lock.unlock();
        logToFile(f, message_level, scope, format, args);
    }
}

pub fn logToFile(
    file: std.Io.File,
    message_level: ipc.LogLevel,
    scope: []const u8,
    comptime format: []const u8,
    args: anytype,
) void {
    var file_buf: [4096]u8 = undefined;
    // streaming, as a positional writer would write at the start of the file
    var wtr = file.writerStreaming(io, &file_buf);
    defer wtr.interface.flush() catch {};
    writeLog(&wtr.interface, message_level, scope, format, args);
}

fn writeLog(
    writer: *std.Io.Writer,
    message_level: ipc.LogLevel,
    scope: []const u8,
    comptime format: []const u8,
    args: anytype,
) void {
    writer.print("{s} {s} ", .{ @tagName(message_level), scope }) catch return;
    writer.print(format ++ "\n", args) catch return;
}

pub const logger = std.log.scoped(.manderrow_agent);

pub const alloc = std.heap.smp_allocator;

comptime {
    if (build_options.host_lib) {
        // export IPC functions
        _ = rs.impl;
    } else {
        // export entrypoints
        switch (builtin.target.os.tag) {
            .windows => {
                _ = windows;
            },
            .macos => {
                @export(&[1]*const fn () callconv(.c) void{entrypoint_macos}, .{
                    .section = "__DATA,__mod_init_func",
                    .name = "init_array",
                });
            },
            .linux => {
                if (builtin.target.abi.isGnu()) {
                    @export(&[1]*const fn (
                        argc: c_int,
                        argv: [*][*:0]u8,
                    ) callconv(.c) void{entrypoint_linux_gnu}, .{
                        .section = ".init_array",
                        .name = "init_array",
                    });
                } else {
                    @compileError("Unsupported target ABI " ++ @tagName(builtin.target.abi));
                }
            },
            else => @compileError("Unsupported target OS " ++ @tagName(builtin.target.os.tag)),
        }
    }

    // export crash function to rust code
    _ = crash;
}

fn entrypoint_linux_gnu(
    argc: c_int,
    argv_ptr: [*][*:0]u8,
) callconv(.c) void {
    if (!builtin.target.abi.isGnu()) {
        @compileError("Unsupported target ABI " ++ @tagName(builtin.target.abi));
    }
    argv = argv_ptr[0..@intCast(argc)];
    entrypoint({});
}

extern fn _NSGetArgc() *c_int;
extern fn _NSGetArgv() *[*][*:0]u8;
extern fn _NSGetEnviron() *[*:null]?[*:0]u8;

fn entrypoint_macos() callconv(.c) void {
    argv = _NSGetArgv().*[0..@intCast(_NSGetArgc().*)];
    entrypoint({});
}

pub fn entrypoint(module: if (builtin.target.os.tag == .windows) std.os.windows.HMODULE else void) void {
    if (builtin.is_test)
        return;

    logger.debug("Agent pre-started", .{});

    std.debug.attachSegfaultHandler();

    logger.debug("Attached segfault handler", .{});

    logger.debug("{f}", .{dump_args});
    logger.debug("{f}", .{dump_env});

    startAgent();

    if (builtin.target.os.tag == .windows) {
        var success = true;
        dll_proxy.loadProxy(module) catch |e| switch (e) {
            error.OutOfMemory => @panic("Out of memory"),
            error.UnsupportedName => success = false,
            else => crash.crash(@src(), "Failed to load actual DLL: {}", .{e}),
        };

        if (success) {
            logger.debug("Loaded proxy", .{});
        } else {
            logger.debug("Unsupported proxy", .{});
        }
    }
}

fn startAgent() void {
    paths.start_time = std.Io.Timestamp.now(io, .real).toMilliseconds();

    var agent_args = Args.extract() catch |e| switch (e) {
        error.Disabled => return,
        else => crash.crash(@src(), "{}", .{e}),
    };
    defer agent_args.deinit();

    const open_log_file_result = if (agent_args.log_to_file) openLogFile(agent_args.logs_dir);

    logger.debug("Parsed arguments", .{});

    logger.debug("{f}", .{dump_args});
    logger.debug("{f}", .{dump_env});

    switch (build_options.ipc_mode) {
        .ipc_channel, .winelib => |t| {
            if (t == .winelib) {
                const path = std.unicode.wtf8ToWtf16LeAllocZ(alloc, agent_args.dlfcn_host_path orelse @panic("Missing required option --dlfcn-host-path")) catch |err| switch (err) {
                    // already validated
                    error.InvalidWtf8 => unreachable,
                    error.OutOfMemory => @panic("Out of memory"),
                };
                defer alloc.free(path);
                rs.impl.init(path, agent_args.agent_host_path orelse @panic("Missing required option --agent-host-path"));
            }

            startIpc(agent_args.c2s_tx);
            logger.debug("Ran Rust-side init", .{});
        },
        .stderr => {},
    }

    open_log_file_result catch |e| {
        logger.err("Failed to open log file: {}", .{e});
    };

    logger.info("Agent started", .{});
    {
        var buf = std.Io.Writer.Allocating.init(alloc);
        defer buf.deinit();

        dumpArgs(&buf.writer) catch {};
        switch (build_options.ipc_mode) {
            .ipc_channel, .winelib => {
                rs.sendLog(.debug, "manderrow_agent", buf.written()) catch |e| logger.warn("{}", .{e});
            },
            .stderr => {
                logger.debug("{s}", .{buf.written()});
            },
        }

        buf.clearRetainingCapacity();

        dumpEnv(&buf.writer) catch {};
        switch (build_options.ipc_mode) {
            .ipc_channel, .winelib => {
                rs.sendLog(.debug, "manderrow_agent", buf.written()) catch |e| logger.warn("{}", .{e});
            },
            .stderr => {
                logger.debug("{s}", .{buf.written()});
            },
        }
    }

    switch (build_options.ipc_mode) {
        .ipc_channel, .winelib => {
            stdio.forwardStdio() catch |e| crash.crash(@src(), "{}", .{e});
        },
        .stderr => {
            // let the wrapper handle it
        },
    }

    logger.debug("Hooked stdio for forwarding", .{});

    interpret_instructions(agent_args.instructions);

    logger.debug("Interpreted instructions", .{});
}

fn openLogFile(logs_dir_override: ?std.Io.Dir) !void {
    const logs_dir = paths.getOrInitLogsDir(logs_dir_override);
    log_file = try logs_dir.createFile(io, &paths.logFileName("log"), .{});
}

fn startIpc(c2s_tx: ?[]const u8) void {
    var error_message_buf: [4096]u8 = undefined;
    var error_buf: rs.ErrorBuffer = .{
        .errno = 0,
        .message_buf = &error_message_buf,
        .message_len = error_message_buf.len,
    };
    switch (rs.manderrow_agent_init(if (c2s_tx) |s| s.ptr else null, if (c2s_tx) |s| s.len else 0, &error_buf)) {
        .Success => {},
        else => @panic(error_message_buf[0..error_buf.message_len]),
    }
}

// make it available to Zig's start.zig
pub const DllMain = if (builtin.target.os.tag == .windows) windows.DllMain;

fn wtf8ToWtf16LeZChecked(wtf16le: [:0]u16, wtf8: []const u8) error{ InvalidWtf8, Overflow }!usize {
    const len = std.unicode.calcWtf16LeLen(wtf8) catch return error.InvalidWtf8;
    if (len > wtf16le.len) {
        return error.Overflow;
    }
    const n = try std.unicode.wtf8ToWtf16Le(wtf16le, wtf8);
    wtf16le[n] = 0;
    return n;
}

noinline fn dumpStackHeight() void {
    const sp = @frameAddress();
    const height = windows.initialFrameAddress - sp;
    logger.debug("Stack height is {} (initial: 0x{x:0>16}, current: 0x{x:0>16})", .{ height, windows.initialFrameAddress, sp });

    const tib = &std.os.windows.teb().NtTib;
    const base = @intFromPtr(tib.StackBase);
    const limit = @intFromPtr(tib.StackLimit);
    logger.debug("Max stack height is {} (base: 0x{x:0>16}, limit: 0x{x:0>16})", .{ base - limit, base, limit });
}

fn interpret_instructions(instructions: []const Args.Instruction) void {
    const PathBuf = if (builtin.target.os.tag == .windows) [std.os.windows.PATH_MAX_WIDE:0]u16 else void;
    var path_buf: if (builtin.target.os.tag == .windows) ?*PathBuf else void = if (builtin.target.os.tag == .windows) null;
    defer if (builtin.target.os.tag == .windows) if (path_buf) |buf| alloc.destroy(buf);
    for (instructions) |insn| {
        switch (insn) {
            .load_library => |ll| {
                logger.debug("Loading library from \"{f}\"", .{std.zig.fmtString(ll.path)});
                switch (builtin.target.os.tag) {
                    .windows => {
                        const buf = path_buf orelse blk: {
                            const buf = alloc.create(PathBuf) catch @panic("Out of memory");
                            path_buf = buf;
                            break :blk buf;
                        };
                        const n = wtf8ToWtf16LeZChecked(buf, ll.path) catch |e| switch (e) {
                            error.InvalidWtf8 => @panic("Invalid --insn-load-library path: invalid WTF-8"),
                            error.Overflow => @panic("Invalid --insn-load-library path: too long"),
                        };
                        std.debug.assert(std.mem.len(@as([*:0]const u16, buf)) == n);
                        if (builtin.target.os.tag == .windows) {
                            dumpStackHeight();
                        }
                        if (util.windows.kernel32.LoadLibraryW(buf) == null) {
                            util.windows.panicWindowsError(@src(), "LoadLibraryW");
                        }
                    },
                    else => {
                        if (std.c.dlopen(ll.path, .{ .LAZY = true }) == null) {
                            const msg = if (std.c.dlerror()) |s| std.mem.span(s) else "No error message";
                            crash.crash(@src(), "dlopen: {s}", .{msg});
                        }
                    },
                }
            },
            .set_var => |sv| {
                const key = sv.kv[0..sv.eq_sign];
                const value = sv.kv[sv.eq_sign + 1 .. :0];
                logger.debug("Setting environment variable {s}=\"{f}\"", .{ key, std.zig.fmtString(value) });
                switch (builtin.target.os.tag) {
                    .windows => {
                        const key_buf = std.unicode.wtf8ToWtf16LeAllocZ(alloc, key) catch |e| switch (e) {
                            error.InvalidWtf8 => @panic("Invalid --insn-set-var key: invalid WTF-8"),
                            error.OutOfMemory => @panic("Out of memory"),
                        };
                        defer alloc.free(key_buf);
                        // Documented max length of environment variable value.
                        var value_buf: [32_767:0]u16 = undefined;
                        _ = wtf8ToWtf16LeZChecked(&value_buf, value) catch |e| switch (e) {
                            error.InvalidWtf8 => @panic("Invalid --insn-set-var value: invalid WTF-8"),
                            error.Overflow => @panic("Invalid --insn-set-var value: too long"),
                        };
                        util.setEnv(key_buf, &value_buf) catch |e| crash.crash(@src(), "{}", .{e});
                    },
                    else => {
                        const key_buf = alloc.dupeSentinel(u8, key, 0) catch @panic("Out of memory");
                        defer alloc.free(key_buf);
                        util.setEnv(key_buf, value) catch |e| crash.crash(@src(), "{}", .{e});
                    },
                }
            },
            .prepend_arg => {
                @panic("TODO: --insn-prepend-arg");
            },
            .append_arg => {
                @panic("TODO: --insn-append-arg");
            },
        }
    }
}

const dump_args = struct {
    pub fn format(_: @This(), writer: *std.Io.Writer) !void {
        dumpArgs(writer) catch {};
    }
}{};

const dump_env = struct {
    pub fn format(_: @This(), writer: *std.Io.Writer) !void {
        dumpEnv(writer) catch {};
    }
}{};

fn dumpArgs(writer: *std.Io.Writer) !void {
    try writer.writeAll("Args:");
    var iter = processArgs().iterateAllocator(alloc) catch |e| {
        try writer.writeAll("Failed to get args (Out of memory)");
        return e;
    };
    defer iter.deinit();
    while (iter.next()) |arg| {
        try writer.print(" \"{f}\"", .{std.zig.fmtString(arg)});
    }
}

fn dumpEnv(writer: *std.Io.Writer) !void {
    var map = processEnviron().createMap(alloc) catch |e| {
        try writer.writeAll("Env: Failed to get env (Out of memory)");
        return e;
    };
    try writer.writeAll("Env: {\n");
    defer map.deinit();
    for (map.keys(), map.values()) |key, value| {
        try writer.print("  {s}=\"{f}\"\n", .{ key, std.zig.fmtString(value) });
    }
    try writer.writeAll("}");
}
