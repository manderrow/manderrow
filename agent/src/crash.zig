const builtin = @import("builtin");
const std = @import("std");

const rs = @import("rs.zig");
const stdio = @import("stdio.zig");

pub fn crash(msg: []const u8, ret_addr: ?usize) noreturn {
    reportCrashToFile(msg, ret_addr);
    reportCrashToStderr(msg, ret_addr);
    rs.sendCrash(msg) catch {};
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
}

var crash_file_truncate = true;
var crash_file_mutex = std.Thread.Mutex.Recursive.init;

fn reportCrashToFile(msg: []const u8, ret_addr: ?usize) void {
    var truncated: bool = undefined;

    // don't allow multiple threads to be dumping a crash at once
    crash_file_mutex.lock();
    defer crash_file_mutex.unlock();

    truncated = crash_file_truncate;
    crash_file_truncate = false;

    var file = std.fs.cwd().createFile("manderrow-agent-crash.txt", .{
        .truncate = truncated,
    }) catch return;
    defer file.close();

    if (!truncated) {
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
    stdio.real_stderr_mutex.lock();
    defer stdio.real_stderr_mutex.unlock();

    std.debug.lockStdErr();
    defer std.debug.unlockStdErr();

    dumpCrashReport((stdio.real_stderr orelse std.io.getStdErr()).writer(), msg, ret_addr);
}
