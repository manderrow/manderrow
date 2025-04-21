const builtin = @import("builtin");
const std = @import("std");

const crash = @import("crash.zig");

pub const std_options: std.Options = .{
    .log_level = .debug,
    .logFn = logFn,
    .enable_segfault_handler = false,
};

pub fn panic(msg: []const u8, error_return_trace: ?*std.builtin.StackTrace, ret_addr: ?usize) noreturn {
    _ = error_return_trace;
    crash.crash(msg, ret_addr);
}

var log_file: ?std.fs.File = null;
var log_file_lock: std.Thread.Mutex.Recursive = .init;

fn logFn(
    comptime message_level: std.log.Level,
    comptime scope: @TypeOf(.enum_literal),
    comptime format: []const u8,
    args: anytype,
) void {
    const level: rs.LogLevel = switch (message_level) {
        .debug => .debug,
        .info => .info,
        .warn => .warn,
        .err => .err,
    };

    logToFile(level, @tagName(scope), format, args);

    const buf = std.fmt.allocPrint(alloc, format, args) catch return;
    defer alloc.free(buf);
    rs.sendLog(level, @tagName(scope), buf) catch return;
}

pub fn logToFile(
    message_level: rs.LogLevel,
    scope: []const u8,
    comptime format: []const u8,
    args: anytype,
) void {
    if (log_file) |f| {
        log_file_lock.lock();
        defer log_file_lock.unlock();
        f.writer().print("{s} {s} ", .{ @tagName(message_level), scope }) catch return;
        f.writer().print(format ++ "\n", args) catch return;
    }
}

pub const logger = std.log.scoped(.manderrow_agent);

pub const alloc = std.heap.smp_allocator;

const dll_proxy = @import("dll_proxy");

const Args = @import("Args.zig");
const rs = @import("rs.zig");
const stdio = @import("stdio.zig");
const util = @import("util.zig");

comptime {
    // export entrypoints
    switch (builtin.os.tag) {
        .windows => {
            _ = windows;
        },
        .macos => {
            @export(&[1]*const fn () callconv(.C) void{entrypoint_macos}, .{
                .section = "__DATA,__mod_init_func",
                .name = "init_array",
            });
        },
        .linux => {
            if (builtin.abi.isGnu()) {
                @export(&[1]*const fn (
                    argc: c_int,
                    argv: [*][*:0]u8,
                ) callconv(.C) void{entrypoint_linux_gnu}, .{
                    .section = ".init_array",
                    .name = "init_array",
                });
            } else {
                @compileError("Unsupported target ABI " ++ @tagName(builtin.abi));
            }
        },
        else => @compileError("Unsupported target OS " ++ @tagName(builtin.os.tag)),
    }

    // export crash function to rust code
    _ = crash;
}

extern fn atexit(f: *const fn () callconv(.C) void) void;
extern fn at_quick_exit(f: *const fn () callconv(.C) void) void;

fn deinit_c() callconv(.C) void {
    // TODO: implement our own IPC that doesn't rely on thread locals so that this won't panic
    // rs.manderrow_agent_send_exit(0, false);
}

fn entrypoint_linux_gnu(
    argc: c_int,
    argv: [*][*:0]u8,
) callconv(.c) void {
    if (!builtin.abi.isGnu()) {
        @compileError("Unsupported target ABI " ++ @tagName(builtin.abi));
    }
    std.os.argv = @constCast(argv[0..@intCast(argc)]);
    entrypoint({});
}

extern fn _NSGetArgc() *c_int;
extern fn _NSGetArgv() *[*][*:0]c_char;

fn entrypoint_macos() callconv(.c) void {
    std.os.argv = _NSGetArgv().*[0..@intCast(_NSGetArgc().*)];
    entrypoint({});
}

fn entrypoint(module: if (builtin.os.tag == .windows) std.os.windows.HMODULE else void) void {
    if (builtin.is_test)
        return;

    log_file = std.fs.cwd().createFile("manderrow-agent.log", .{}) catch null;

    logger.debug("Agent pre-started", .{});

    std.debug.attachSegfaultHandler();

    logger.debug("Attached segfault handler", .{});

    logger.debug("{}", .{dump_args});
    logger.debug("{}", .{dump_env});

    if (builtin.os.tag != .windows) {
        atexit(deinit_c);

        logger.debug("Set atexit hook", .{});

        at_quick_exit(deinit_c);

        logger.debug("Set at_quick_exit hook", .{});
    }

    startAgent();

    if (builtin.os.tag == .windows) {
        var success = true;
        dll_proxy.loadProxy(module) catch |e| switch (e) {
            error.OutOfMemory => @panic("Out of memory"),
            error.UnsupportedName => success = false,
            else => std.debug.panic("Failed to load actual DLL: {}", .{e}),
        };

        if (success) {
            logger.debug("Loaded proxy", .{});
        } else {
            logger.debug("Unsupported proxy", .{});
        }
    }
}

fn startAgent() void {
    var args = Args.extract() catch |e| switch (e) {
        error.Disabled => return,
        else => std.debug.panic("{}", .{e}),
    };
    defer args.deinit();

    logger.debug("Parsed arguments", .{});

    var error_message_buf: [4096]u8 = undefined;
    var error_buf: rs.ErrorBuffer = .{
        .errno = 0,
        .message_buf = &error_message_buf,
        .message_len = error_message_buf.len,
    };
    switch (rs.manderrow_agent_init(if (args.c2s_tx) |s| s.ptr else null, if (args.c2s_tx) |s| s.len else 0, &error_buf)) {
        .Success => {},
        else => |_| {
            crash.crash(error_message_buf[0..error_buf.message_len], null);
        },
    }

    logger.debug("Ran Rust-side init", .{});

    logger.info("Agent started", .{});
    {
        var buf = std.ArrayListUnmanaged(u8){};
        defer buf.deinit(alloc);

        dumpArgs(buf.writer(alloc)) catch {};
        rs.sendLog(.debug, "manderrow_agent", buf.items) catch |e| std.debug.panic("{}", .{e});

        buf.clearRetainingCapacity();

        dumpEnv(buf.writer(alloc)) catch {};
        rs.sendLog(.debug, "manderrow_agent", buf.items) catch |e| std.debug.panic("{}", .{e});
    }

    stdio.forwardStdio() catch |e| std.debug.panic("{}", .{e});

    logger.debug("Hooked stdio for forwarding", .{});

    interpret_instructions(args.instructions);

    logger.debug("Interpreted instructions", .{});
}

const windows = struct {
    const FdwReason = enum(std.os.windows.DWORD) {
        PROCESS_DETACH = 0,
        PROCESS_ATTACH = 1,
        THREAD_ATTACH = 2,
        THREAD_DETACH = 3,
        _,
    };

    noinline fn DllMain(
        hInstDll: std.os.windows.HINSTANCE,
        fdwReasonRaw: u32,
        _: std.os.windows.LPVOID,
    ) std.os.windows.BOOL {
        const module: std.os.windows.HMODULE = @ptrCast(hInstDll);

        const fdwReason: FdwReason = @enumFromInt(fdwReasonRaw);

        if (fdwReason != .PROCESS_ATTACH) {
            return std.os.windows.TRUE;
        }

        entrypoint(module);

        return std.os.windows.TRUE;
    }
};

// make it available to Zig's start.zig
pub const DllMain = if (builtin.os.tag == .windows) windows.DllMain;

fn wtf8ToWtf16LeZChecked(wtf16le: [:0]u16, wtf8: []const u8) error{ InvalidWtf8, Overflow }!usize {
    if (try std.unicode.checkWtf8ToWtf16LeOverflow(wtf8, wtf16le)) {
        return error.Overflow;
    }
    const n = try std.unicode.wtf8ToWtf16Le(wtf16le, wtf8);
    wtf16le[n] = 0;
    return n;
}

fn interpret_instructions(instructions: []const Args.Instruction) void {
    for (instructions) |insn| {
        switch (insn) {
            .load_library => |ll| {
                logger.debug("Loading library from \"{}\"", .{std.zig.fmtEscapes(ll.path)});
                switch (builtin.os.tag) {
                    .windows => {
                        var buf: [std.os.windows.MAX_PATH:0]u16 = undefined;
                        const n = wtf8ToWtf16LeZChecked(&buf, ll.path) catch |e| switch (e) {
                            error.InvalidWtf8 => @panic("Invalid --insn-load-library path: invalid WTF-8"),
                            error.Overflow => @panic("Invalid --insn-load-library path: too long"),
                        };
                        std.debug.assert(std.mem.len(@as([*:0]const u16, &buf)) == n);
                        if (std.os.windows.kernel32.LoadLibraryW(&buf) == null) {
                            util.windows.panicWindowsError("LoadLibraryW");
                        }
                    },
                    else => {
                        if (std.c.dlopen(ll.path, .{}) == null) {
                            const msg = if (std.c.dlerror()) |s| std.mem.span(s) else "No error message";
                            std.debug.panic("dlopen: {s}", .{msg});
                        }
                    },
                }
            },
            .set_var => |sv| {
                const key = sv.kv[0..sv.eq_sign];
                const value = sv.kv[sv.eq_sign + 1 .. :0];
                logger.debug("Setting environment variable {s}=\"{}\"", .{ key, std.zig.fmtEscapes(value) });
                switch (builtin.os.tag) {
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
                        util.setEnv(key_buf, &value_buf) catch |e| std.debug.panic("{}", .{e});
                    },
                    else => {
                        const key_buf = alloc.dupeZ(u8, key) catch @panic("Out of memory");
                        defer alloc.free(key_buf);
                        util.setEnv(key_buf, value) catch |e| std.debug.panic("{}", .{e});
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
    pub fn format(_: @This(), comptime fmt: []const u8, _: std.fmt.FormatOptions, writer: anytype) !void {
        if (fmt.len != 0) @compileError("Unsupported format specifier: " ++ fmt);
        dumpArgs(writer) catch {};
    }
}{};

const dump_env = struct {
    pub fn format(_: @This(), comptime fmt: []const u8, _: std.fmt.FormatOptions, writer: anytype) !void {
        if (fmt.len != 0) @compileError("Unsupported format specifier: " ++ fmt);
        dumpEnv(writer) catch {};
    }
}{};

fn dumpArgs(writer: anytype) !void {
    try writer.writeAll("Args:");
    var iter = std.process.argsWithAllocator(alloc) catch |e| {
        try writer.writeAll("Failed to get args (Out of memory)");
        return e;
    };
    defer iter.deinit();
    while (iter.next()) |arg| {
        try writer.print(" \"{}\"", .{std.zig.fmtEscapes(arg)});
    }
}

fn dumpEnv(writer: anytype) !void {
    var map = std.process.getEnvMap(alloc) catch |e| {
        try writer.writeAll("Env: Failed to get env (Out of memory)");
        return e;
    };
    try writer.writeAll("Env: {\n");
    defer map.deinit();
    var iter = map.iterator();
    while (iter.next()) |entry| {
        try writer.print("  {s}=\"{}\"\n", .{ entry.key_ptr.*, std.zig.fmtEscapes(entry.value_ptr.*) });
    }
    try writer.writeAll("}");
}
