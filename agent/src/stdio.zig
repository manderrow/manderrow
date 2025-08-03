const builtin = @import("builtin");
const std = @import("std");

const root = @import("root.zig");
const alloc = root.alloc;
const logger = root.logger;
const ipc = @import("ipc.zig");
const rs = @import("rs.zig");

pub var real_stderr: ?std.fs.File = null;
pub var real_stderr_mutex = std.Thread.Mutex.Recursive.init;

pub fn forwardStdio() !void {
    switch (builtin.os.tag) {
        .windows => {
            real_stderr = .{ .handle = GetStdHandlePtr(.err).* };

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
                // this function in the std lib really ought to take a bool for
                // bInheritHandle and not take nLength at all since it is unused.
                try std.os.windows.CreatePipe(&rd, &wr, &.{
                    .nLength = undefined,
                    .lpSecurityDescriptor = null,
                    .bInheritHandle = std.os.windows.FALSE,
                });
                GetStdHandlePtr(channel.id).* = wr;
                const thread = try std.Thread.spawn(
                    .{},
                    forwardFromPipe,
                    .{ channel.channel, std.fs.File{ .handle = rd } },
                );
                defer thread.detach();
                thread.setName("manderrow-std" ++ @tagName(channel.channel) ++ "-forwarder") catch {};
            }
        },
        else => {
            real_stderr = .{ .handle = try std.posix.dup(std.posix.STDERR_FILENO) };

            const Channel = struct {
                channel: ipc.StandardOutputChannel,
                fd: std.posix.fd_t,
            };
            inline for ([_]Channel{
                .{ .channel = .out, .fd = std.posix.STDOUT_FILENO },
                .{ .channel = .err, .fd = std.posix.STDERR_FILENO },
            }) |channel| {
                const pipe = try std.posix.pipe();
                try std.posix.dup2(pipe[1], channel.fd);
                const thread = try std.Thread.spawn(
                    .{},
                    forwardFromPipe,
                    .{ channel.channel, std.fs.File{ .handle = pipe[0] } },
                );
                defer thread.detach();
                thread.setName("manderrow-std" ++ @tagName(channel.channel) ++ "-forwarder") catch {};
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

fn forwardFromPipe(channel: ipc.StandardOutputChannel, pipe: std.fs.File) void {
    defer pipe.close();
    var rdr_buf: [4096]u8 = undefined;
    var rdr = pipe.reader(&rdr_buf);
    var buf = std.io.Writer.Allocating.initCapacity(alloc, 256) catch |e| {
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
            rdr.interface.seek += 1;
        }

        var line = buf.getWritten();
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
