const builtin = @import("builtin");
const std = @import("std");

pub const posix = @import("util/posix.zig");
pub const windows = @import("util/windows.zig");

pub const os_char = if (builtin.os.tag == .windows) std.os.windows.WCHAR else u8;

pub fn setEnvComptimeKey(comptime key: [:0]const u8, value: ?[:0]const os_char) void {
    return setEnv(std.unicode.utf8ToUtf16LeStringLiteral(key), value);
}

pub fn setEnv(key: [:0]const os_char, value: ?[:0]const os_char) !void {
    switch (builtin.os.tag) {
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
