const builtin = @import("builtin");
const std = @import("std");

var panicked = std.atomic.Value(bool).init(false);

pub fn panic(msg: []const u8, error_return_trace: ?*std.builtin.StackTrace, ret_addr: ?usize) noreturn {
    _ = error_return_trace;
    if (!panicked.swap(true, .monotonic)) {
        rs.report_crash(msg) catch {};
    }
    std.debug.defaultPanic(msg, ret_addr);
}

pub const alloc = std.heap.c_allocator;

const dll_proxy = @import("dll_proxy");

const Args = @import("Args.zig");
const rs = @import("rs.zig");
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
}

extern fn atexit(f: *const fn () callconv(.C) void) void;
extern fn at_quick_exit(f: *const fn () callconv(.C) void) void;

fn deinit_c() callconv(.C) void {
    // TODO: implement our own IPC that doesn't rely on thread locals so that this won't panic
    rs.manderrow_agent_deinit(false);
}

fn entrypoint_linux_gnu(
    argc: c_int,
    argv: [*][*:0]u8,
) callconv(.c) void {
    if (!builtin.abi.isGnu()) {
        @compileError("Unsupported target ABI " ++ @tagName(builtin.abi));
    }
    std.os.argv = @constCast(argv[0..@intCast(argc)]);
    entrypoint();
}

extern fn _NSGetArgc() *c_int;
extern fn _NSGetArgv() *[*][*:0]c_char;

fn entrypoint_macos() callconv(.c) void {
    std.os.argv = _NSGetArgv().*[0..@intCast(_NSGetArgc().*)];
    entrypoint();
}

fn entrypoint() void {
    if (builtin.is_test)
        return;

    const log_file: ?std.fs.File = std.fs.cwd().createFile("manderrow-agent.log", .{}) catch null;
    defer if (log_file) |f| f.close();

    if (log_file) |f| {
        dumpArgs(f.writer()) catch {};
        f.writeAll("\n") catch {};
        dumpEnv(f.writer()) catch {};
        f.writeAll("\n") catch {};
    }

    if (builtin.os.tag != .windows) {
        atexit(deinit_c);

        if (log_file) |f| {
            f.writeAll("Set atexit hook\n") catch {};
        }

        at_quick_exit(deinit_c);

        if (log_file) |f| {
            f.writeAll("Set at_quick_exit hook\n") catch {};
        }
    }

    var args = Args.extract() catch |e| switch (e) {
        error.Disabled => return,
        else => std.debug.panic("{}", .{e}),
    };
    defer args.deinit();

    if (log_file) |f| {
        f.writeAll("Parsed arguments\n") catch {};
    }

    rs.manderrow_agent_init(if (args.c2s_tx) |s| s.ptr else null, if (args.c2s_tx) |s| s.len else 0);

    if (log_file) |f| {
        f.writeAll("Ran Rust-side init\n") catch {};
    }

    rs.sendLog(.info, "manderrow_agent", "Agent started") catch |e| std.debug.panic("{}", .{e});
    {
        var buf = std.ArrayListUnmanaged(u8){};
        defer buf.deinit(alloc);

        dumpArgs(buf.writer(alloc)) catch {};
        rs.sendLog(.debug, "manderrow_agent", buf.items) catch |e| std.debug.panic("{}", .{e});

        buf.clearRetainingCapacity();

        dumpEnv(buf.writer(alloc)) catch {};
        rs.sendLog(.debug, "manderrow_agent", buf.items) catch |e| std.debug.panic("{}", .{e});
    }

    @import("stdio.zig").forwardStdio() catch |e| std.debug.panic("{}", .{e});

    if (log_file) |f| {
        f.writeAll("Hooked stdio for forwarding\n") catch {};
    }

    interpret_instructions(args.instructions);

    if (log_file) |f| {
        f.writeAll("Interpreted instructions\n") catch {};
    }
}

const windows = struct {
    const FdwReason = enum(std.os.windows.DWORD) {
        PROCESS_DETACH = 0,
        PROCESS_ATTACH = 1,
        THREAD_ATTACH = 2,
        THREAD_DETACH = 3,
        _,
    };

    export fn DllMain(
        hInstDll: std.os.windows.HINSTANCE,
        fdwReasonRaw: u32,
        _: std.os.windows.LPVOID,
    ) callconv(.winapi) std.os.windows.BOOL {
        const module: std.os.windows.HMODULE = @ptrCast(hInstDll);

        const fdwReason: FdwReason = @enumFromInt(fdwReasonRaw);

        if (fdwReason != .PROCESS_ATTACH) {
            return std.os.windows.TRUE;
        }

        entrypoint();

        dll_proxy.loadProxy(module) catch |e| switch (e) {
            error.OutOfMemory => @panic("Out of memory"),
            error.UnsupportedName => {},
            else => std.debug.panic("Failed to load actual DLL: {}", .{e}),
        };

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
                switch (builtin.os.tag) {
                    .windows => {
                        var buf: [std.os.windows.MAX_PATH:0]u16 = undefined;
                        _ = wtf8ToWtf16LeZChecked(&buf, ll.path) catch |e| switch (e) {
                            error.InvalidWtf8 => @panic("Invalid --insn-load-library path: invalid WTF-8"),
                            error.Overflow => @panic("Invalid --insn-load-library path: too long"),
                        };
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
            .set_var => |ll| {
                const key = ll.kv[0..ll.eq_sign];
                const value = ll.kv[ll.eq_sign + 1 .. :0];
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
