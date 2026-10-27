const builtin = @import("builtin");
const std = @import("std");

const io = @import("root.zig").io;

pub const posix = @import("util/posix.zig");
pub const windows = @import("util/windows.zig");

pub const os_char = if (builtin.target.os.tag == .windows) std.os.windows.WCHAR else u8;

pub fn setEnvComptimeKey(comptime key: [:0]const u8, value: ?[:0]const os_char) void {
    return setEnv(std.unicode.utf8ToUtf16LeStringLiteral(key), value);
}

pub fn setEnv(key: [:0]const os_char, value: ?[:0]const os_char) !void {
    switch (builtin.target.os.tag) {
        .windows => {
            windows.SetEnvironmentVariable(key, value orelse null);
        },
        else => {
            if (value) |v| {
                try posix.setenv(key, v, true);
            } else {
                try posix.unsetenv(key);
            }
        },
    }
}

/// A mutex that the thread holding it can lock again, as std no longer has one.
pub const RecursiveMutex = struct {
    mutex: std.Io.Mutex,
    thread_id: std.Thread.Id,
    lock_count: usize,

    pub const init: RecursiveMutex = .{
        .mutex = .init,
        .thread_id = invalid_thread_id,
        .lock_count = 0,
    };

    pub fn lock(r: *RecursiveMutex) void {
        const current_thread_id = std.Thread.getCurrentId();
        if (@atomicLoad(std.Thread.Id, &r.thread_id, .unordered) != current_thread_id) {
            r.mutex.lockUncancelable(io);
            std.debug.assert(r.lock_count == 0);
            @atomicStore(std.Thread.Id, &r.thread_id, current_thread_id, .unordered);
        }
        r.lock_count += 1;
    }

    pub fn unlock(r: *RecursiveMutex) void {
        r.lock_count -= 1;
        if (r.lock_count == 0) {
            @atomicStore(std.Thread.Id, &r.thread_id, invalid_thread_id, .unordered);
            r.mutex.unlock(io);
        }
    }

    const invalid_thread_id: std.Thread.Id = std.math.maxInt(std.Thread.Id);
};
