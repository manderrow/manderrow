const builtin = @import("builtin");
const std = @import("std");

const build_zig_zon = @import("build.zig.zon");

const rs = @import("rs.zig");
const stdio = @import("stdio.zig");

threadlocal var thread_crashed = false;

pub fn crash(msg: []const u8, ret_addr: ?usize) noreturn {
    if (!thread_crashed) {
        thread_crashed = true;
        reportCrashToFile(msg, ret_addr);
        reportCrashToStderr(msg, ret_addr);
        rs.sendCrash(msg) catch {};
    } else {
        // we don't want to attempt reporting the crash recursively, so just emit a
        // breakpoint so that the problem can be investigated with a debugger.
        @breakpoint();
    }
    std.posix.abort();
}

export fn manderrow_agent_crash(msg_ptr: [*]const u8, msg_len: usize) noreturn {
    crash(msg_ptr[0..msg_len], @returnAddress());
}

fn dumpCrashReport(writer: anytype, msg: []const u8, ret_addr: ?usize) void {
    writer.print(
        \\{s}
        \\
        \\Backtrace:
        \\
    , .{msg}) catch return;
    if (builtin.strip_debug_info) {
        writer.print("Unable to dump stack trace: debug info stripped\n", .{}) catch return;
        return;
    }
    const debug_info = std.debug.getSelfDebugInfo() catch |err| {
        writer.print("Unable to dump stack trace: Unable to open debug info: {s}\n", .{@errorName(err)}) catch return;
        return;
    };
    std.debug.writeCurrentStackTrace(writer, debug_info, .no_color, ret_addr) catch |err| {
        writer.print("Unable to dump stack trace: {s}\n", .{@errorName(err)}) catch return;
        return;
    };
    writer.print(std.fmt.comptimePrint(
        \\
        \\Version: {s}
        \\
    , .{build_zig_zon.version}), .{}) catch return;
    if (hashSelfExe() catch |e| blk: {
        writer.print("Binary hash: error {}", .{e}) catch return;
        break :blk null;
    }) |hash| {
        writer.print("Binary hash: {}", .{std.fmt.fmtSliceHexLower(&hash)}) catch return;
    }
}

fn hashSelfExe() ![32]u8 {
    var f = try std.fs.openSelfExe(.{});
    var hsr = std.crypto.hash.Blake3.init(.{});
    var buf: [4096]u8 = undefined;
    while (true) {
        const n = try f.read(&buf);
        if (n == 0) break;
        hsr.update(buf[0..n]);
    }
    var hash: [32]u8 = undefined;
    hsr.final(&hash);
    return hash;
}

var crash_file_truncate = true;
var crash_file_mutex: std.Thread.Mutex = .{};

fn reportCrashToFile(msg: []const u8, ret_addr: ?usize) void {
    // don't allow multiple threads to be dumping a crash at once
    crash_file_mutex.lock();
    defer crash_file_mutex.unlock();

    const truncate = crash_file_truncate;
    crash_file_truncate = false;

    const paths = @import("paths.zig");
    const logs_dir = paths.getOrInitLogsDir(null);
    var file = switch (builtin.os.tag) {
        .windows => logs_dir.createFileW(&paths.logFileName("crash"), .{
            .truncate = false,
        }),
        else => logs_dir.createFileZ(&paths.logFileName("crash"), .{
            .truncate = false,
        }),
    } catch return;
    defer file.close();

    if (!truncate) {
        file.seekFromEnd(0) catch return;
        file.writeAll(
            \\
            \\
            \\==== Next crash ====
            \\
            \\
        ) catch return;
    }

    dumpCrashReport(file.writer(), msg, ret_addr);
}

fn reportCrashToStderr(msg: []const u8, ret_addr: ?usize) void {
    if (stdio.real_stderr != null) stdio.real_stderr_mutex.lock();
    defer if (stdio.real_stderr != null) stdio.real_stderr_mutex.unlock();

    if (stdio.real_stderr == null) std.debug.lockStdErr();
    defer if (stdio.real_stderr == null) std.debug.unlockStdErr();

    dumpCrashReport((stdio.real_stderr orelse std.io.getStdErr()).writer(), msg, ret_addr);
}
