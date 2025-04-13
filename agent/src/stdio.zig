const builtin = @import("builtin");
const std = @import("std");

const alloc = @import("root.zig").alloc;

const rs = @import("rs.zig");

pub var real_stderr: ?std.fs.File = null;
pub var real_stderr_mutex = std.Thread.Mutex.Recursive.init;

pub fn forwardStdio() !void {
    switch (builtin.os.tag) {
        .windows => {
            real_stderr = std.fs.File{ .handle = GetStdHandlePtr(.err).* };
            const Channel = struct {
                channel: rs.StandardOutputChannel,
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
                thread.setName("manderrow-std" ++ @tagName(channel.channel) ++ "-forwarder") catch {};
            }
        },
        else => {
            const Channel = struct {
                channel: rs.StandardOutputChannel,
                fd: std.posix.fd_t,
            };
            real_stderr = std.fs.File{ .handle = try std.posix.dup(std.posix.STDERR_FILENO) };
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

fn forwardFromPipe(channel: rs.StandardOutputChannel, pipe: std.fs.File) void {
    defer pipe.close();
    var rdr = std.io.bufferedReader(pipe.reader());
    var buf = std.ArrayListUnmanaged(u8){};
    defer buf.deinit(alloc);
    while (true) {
        defer buf.clearRetainingCapacity();

        rdr.reader().streamUntilDelimiter(buf.writer(alloc), '\n', null) catch |e| switch (e) {
            error.EndOfStream => {},
            else => {
                real_stderr_mutex.lock();
                defer real_stderr_mutex.unlock();
                std.fmt.format(real_stderr.?.writer(), "Error in stdio forwarder: {}", .{e}) catch {};
                return;
            },
        };

        if (buf.getLastOrNull()) |c| {
            if (c == '\r') {
                _ = buf.pop().?;
            }
        }

        if (tryForwardLineAsLogRecord(buf.items)) {
            continue;
        }

        // forward normally
        rs.sendOutputLine(channel, buf.items);
    }
}

fn tryForwardLineAsLogRecord(line: []const u8) bool {
    var iter = std.mem.splitScalar(u8, line, ' ');
    const level = std.meta.stringToEnum(rs.LogLevel, iter.first()) orelse {
        return false;
    };

    const scope = iter.next() orelse return false;
    const msg = iter.rest();

    rs.sendLog(level, scope, msg) catch return false;

    return true;
}
