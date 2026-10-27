const builtin = @import("builtin");
const std = @import("std");

const root = @import("root.zig");
const alloc = root.alloc;
const io = root.io;
const logger = root.logger;
const ipc = @import("ipc.zig");
const rs = @import("rs.zig");
const util = @import("util.zig");

pub var real_stderr: ?std.Io.File = null;
pub var real_stderr_mutex: util.RecursiveMutex = .init;

pub fn forwardStdio() !void {
    switch (builtin.target.os.tag) {
        .windows => {
            real_stderr = .{ .handle = GetStdHandlePtr(.err).*, .flags = .{ .nonblocking = false } };

            const Channel = struct {
                channel: ipc.StandardOutputChannel,
                id: StdHandle,
            };
            inline for ([_]Channel{
                .{ .channel = .out, .id = .out },
                .{ .channel = .err, .id = .err },
            }) |channel| {
                var rd: std.os.windows.HANDLE = undefined;
                var wr: std.os.windows.HANDLE = undefined;
                // without security attributes, the handles aren't inherited
                if (!util.windows.kernel32.CreatePipe(&rd, &wr, null, 0).toBool()) {
                    return std.os.windows.unexpectedError(std.os.windows.GetLastError());
                }
                GetStdHandlePtr(channel.id).* = wr;
                const thread = try std.Thread.spawn(
                    .{},
                    forwardFromPipe,
                    .{ channel.channel, std.Io.File{ .handle = rd, .flags = .{ .nonblocking = false } } },
                );
                defer thread.detach();
                thread.setName(io, "manderrow-std" ++ @tagName(channel.channel) ++ "-forwarder") catch {};
            }
        },
        else => {
            const stderr_fd = std.c.dup(std.posix.STDERR_FILENO);
            if (stderr_fd == -1) {
                return std.posix.unexpectedErrno(std.posix.errno(stderr_fd));
            }
            real_stderr = .{ .handle = stderr_fd, .flags = .{ .nonblocking = false } };

            const Channel = struct {
                channel: ipc.StandardOutputChannel,
                fd: std.posix.fd_t,
            };
            inline for ([_]Channel{
                .{ .channel = .out, .fd = std.posix.STDOUT_FILENO },
                .{ .channel = .err, .fd = std.posix.STDERR_FILENO },
            }) |channel| {
                const pipe = try std.Io.Threaded.pipe2(.{});
                try std.Io.Threaded.dup2(pipe[1], channel.fd);
                const thread = try std.Thread.spawn(
                    .{},
                    forwardFromPipe,
                    .{ channel.channel, std.Io.File{ .handle = pipe[0], .flags = .{ .nonblocking = false } } },
                );
                defer thread.detach();
                thread.setName(io, "manderrow-std" ++ @tagName(channel.channel) ++ "-forwarder") catch {};
            }
        },
    }
}

const StdHandle = enum { in, out, err };

fn GetStdHandlePtr(handle_id: StdHandle) *std.os.windows.HANDLE {
    const params = std.os.windows.peb().ProcessParameters;
    return switch (handle_id) {
        .in => &params.hStdInput,
        .out => &params.hStdOutput,
        .err => &params.hStdError,
    };
}

fn forwardFromPipe(channel: ipc.StandardOutputChannel, pipe: std.Io.File) void {
    defer pipe.close(io);
    var rdr_buf: [4096]u8 = undefined;
    var rdr = pipe.readerStreaming(io, &rdr_buf);
    var buf = std.Io.Writer.Allocating.initCapacity(alloc, 256) catch |e| {
        logger.err("Error in stdio forwarder: {}", .{e});
        return;
    };
    defer buf.deinit();
    while (true) {
        defer buf.clearRetainingCapacity();

        _ = rdr.interface.streamDelimiterEnding(&buf.writer, '\n') catch |e| {
            logger.err("Error in stdio forwarder: {}", .{e});
            return;
        };
        if (rdr.interface.bufferedLen() != 0) {
            std.debug.assert(rdr.interface.buffered()[0] == '\n');
            rdr.interface.toss(1);
        }

        var line = buf.written();
        if (line.len != 0 and line[line.len - 1] == '\r') {
            line = line[0 .. line.len - 1];
        }

        if (tryForwardLineAsLogRecord(line)) {
            continue;
        }

        // forward stdout and stderr to our log file
        root.logToLogFile(.info, @tagName(channel), "{s}", .{line});

        // forward normally
        rs.sendOutputLine(channel, line);
    }
}

fn tryForwardLineAsLogRecord(line: []const u8) bool {
    var iter = std.mem.splitScalar(u8, line, ' ');
    const level = std.meta.stringToEnum(ipc.LogLevel, iter.first()) orelse {
        return false;
    };

    const scope = iter.next() orelse return false;
    const msg = iter.rest();

    // forward other DLLs logging to our log file
    root.logToLogFile(level, scope, "{s}", .{msg});

    rs.sendLog(level, scope, msg) catch return false;

    return true;
}
