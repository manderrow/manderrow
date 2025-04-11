const builtin = @import("builtin");
const std = @import("std");

const dll_proxy = @import("dll_proxy");

comptime {
    // export entrypoints
    switch (builtin.os.tag) {
        .windows => {
            _ = windows;
        },
        else => {
            @export(&[1]*const fn () callconv(.C) void{entrypoint_c}, .{
                .section = if (builtin.os.tag == .macos) "__DATA,__mod_init_func" else ".init_array",
                .name = "init_array",
            });
        },
    }
}

extern fn atexit(f: *const fn () callconv(.C) void) void;
extern fn at_quick_exit(f: *const fn () callconv(.C) void) void;

extern fn manderrow_agent_init() void;
extern fn manderrow_agent_deinit(send_exit: bool) void;

fn deinit_c() callconv(.C) void {
    // TODO: implement our own IPC that doesn't rely on thread locals so that this won't panic
    manderrow_agent_deinit(false);
}

fn entrypoint_c() callconv(.c) void {
    entrypoint();
}

fn entrypoint() void {
    if (builtin.is_test)
        return;

    const log_file: ?std.fs.File = std.fs.cwd().createFile("manderrow-agent.log", .{}) catch null;
    defer if (log_file) |f| f.close();

    if (log_file) |f| {
        {
            // FIXME: this is empty on posix systems
            f.writeAll("Args:") catch {};
            var iter = std.process.args();
            while (iter.next()) |arg| {
                std.fmt.format(f.writer(), " {s}", .{arg}) catch {};
            }
            f.writeAll("\n") catch {};
        }
        dump_env: {
            f.writeAll("Env: {\n") catch {};
            var map = std.process.getEnvMap(std.heap.c_allocator) catch break :dump_env;
            defer map.deinit();
            var iter = map.iterator();
            while (iter.next()) |entry| {
                std.fmt.format(f.writer(), "  {s}=\"{}\"\n", .{ entry.key_ptr.*, std.zig.fmtEscapes(entry.value_ptr.*) }) catch {};
            }
            f.writeAll("}\n") catch {};
        }
    }

    atexit(deinit_c);

    if (log_file) |f| {
        f.writeAll("Set atexit hook\n") catch {};
    }

    at_quick_exit(deinit_c);

    if (log_file) |f| {
        f.writeAll("Set at_quick_exit hook\n") catch {};
    }

    manderrow_agent_init();

    if (log_file) |f| {
        f.writeAll("Ran Rust-side init\n") catch {};
    }
}

const windows = struct {
    const LoadReason = enum(std.os.windows.DWORD) {
        PROCESS_DETACH = 0,
        PROCESS_ATTACH = 1,
        THREAD_ATTACH = 2,
        THREAD_DETACH = 3,
    };

    export fn DllEntry(
        hInstDll: std.os.windows.HINSTANCE,
        reasonForDllLoad: LoadReason,
        _: std.os.windows.LPVOID,
    ) callconv(.winapi) std.os.windows.BOOL {
        const module: std.os.windows.HMODULE = @ptrCast(hInstDll);

        if (reasonForDllLoad != LoadReason.PROCESS_ATTACH) {
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
