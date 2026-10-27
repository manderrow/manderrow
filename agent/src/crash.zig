const builtin = @import("builtin");
const std = @import("std");

const build_zig_zon = @import("build.zig.zon");
const build_options = @import("build_options");

const io = @import("root.zig").io;
const rs = @import("rs.zig");
const stdio = @import("stdio.zig");

threadlocal var thread_crashed = false;

pub fn panic(msg: []const u8, ret_addr: ?usize) noreturn {
    if (!thread_crashed) {
        thread_crashed = true;
        reportCrashToFile(msg, ret_addr);
        reportCrashToStderr(msg, ret_addr);
        if (build_options.ipc_mode == .ipc_channel) {
            rs.sendCrash(msg) catch {};
        }
    } else {
        // we don't want to attempt reporting the crash recursively, so just emit a
        // breakpoint so that the problem can be investigated with a debugger.
        @breakpoint();
    }
    std.process.abort();
}

const FormatSourceLocation = struct {
    src: std.lang.SourceLocation,

    pub fn format(self: FormatSourceLocation, writer: *std.Io.Writer) !void {
        const src = self.src;
        return writer.print("at {s}:{s}:{}:{}:{s}: ", .{ src.module, src.file, src.line, src.column, src.fn_name });
    }
};

pub fn crash(src: std.lang.SourceLocation, comptime fmt: []const u8, args: anytype) noreturn {
    std.debug.panic("{f}" ++ fmt, .{FormatSourceLocation{ .src = src }} ++ args);
}

export fn manderrow_agent_crash(msg_ptr: [*]const u8, msg_len: usize) noreturn {
    @panic(msg_ptr[0..msg_len]);
}

/// The writer should be unbuffered so that as much is written as possible in case of hard failure,
/// and to slightly improve resistance against stack overflows.
fn dumpCrashReport(writer: *std.Io.Writer, msg: []const u8, ret_addr: ?usize) void {
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
    std.debug.writeCurrentStackTrace(.{ .first_address = ret_addr }, .{ .writer = writer, .mode = .no_color }) catch |err| {
        writer.print("Unable to dump stack trace: {s}\n", .{@errorName(err)}) catch return;
        return;
    };
    writer.print(std.fmt.comptimePrint(
        \\
        \\Version: {s}
        \\
    , .{build_zig_zon.version}), .{}) catch return;
    if (hashSelfExe() catch |e| blk: {
        writer.print("Executable hash: error {}", .{e}) catch return;
        break :blk null;
    }) |hash| {
        writer.writeAll("Executable hash: ") catch return;
        writer.printHex(&hash, .lower) catch return;
    }
    // TODO: hash agent, host agent and host_dlfcn libraries
}

fn hashSelfExe() ![32]u8 {
    const file = try std.process.openExecutable(io, .{});
    defer file.close(io);
    var rdr = file.readerStreaming(io, &.{});
    // the reader reads into the writer's buffer
    var buf: [4096]u8 = undefined;
    var hsr: std.Io.Writer.Hashing(std.crypto.hash.Blake3) = .init(&buf);
    _ = rdr.interface.streamRemaining(&hsr.writer) catch |e| switch (e) {
        error.ReadFailed => return rdr.err.?,
        // hashing can't fail
        error.WriteFailed => unreachable,
    };
    hsr.writer.flush() catch unreachable;
    var hash: [32]u8 = undefined;
    hsr.hasher.final(&hash);
    return hash;
}

var crash_file_truncate = true;
var crash_file_mutex: std.Io.Mutex = .init;

fn reportCrashToFile(msg: []const u8, ret_addr: ?usize) void {
    // don't allow multiple threads to be dumping a crash at once
    crash_file_mutex.lockUncancelable(io);
    defer crash_file_mutex.unlock(io);

    const truncate = crash_file_truncate;
    crash_file_truncate = false;

    const paths = @import("paths.zig");
    const logs_dir = paths.getOrInitLogsDir(null);
    var file = logs_dir.createFile(io, &paths.logFileName("crash"), .{
        .truncate = truncate,
    }) catch return;
    defer file.close(io);

    // see doc comment on dumpCrashReport for unbuffered rationale
    var wtr = file.writer(io, &.{});

    if (!truncate) {
        wtr.seekTo(file.length(io) catch return) catch return;
        wtr.interface.writeAll(
            \\
            \\
            \\==== Next crash ====
            \\
            \\
        ) catch return;
    }

    dumpCrashReport(&wtr.interface, msg, ret_addr);
}

fn reportCrashToStderr(msg: []const u8, ret_addr: ?usize) void {
    if (stdio.real_stderr) |real_stderr| {
        stdio.real_stderr_mutex.lock();
        defer stdio.real_stderr_mutex.unlock();

        // see doc comment on dumpCrashReport for unbuffered rationale
        var wtr = real_stderr.writerStreaming(io, &.{});
        dumpCrashReport(&wtr.interface, msg, ret_addr);
    } else {
        // see doc comment on dumpCrashReport for unbuffered rationale
        const stderr = std.debug.lockStderr(&.{});
        defer std.debug.unlockStderr();
        dumpCrashReport(&stderr.file_writer.interface, msg, ret_addr);
    }
}
