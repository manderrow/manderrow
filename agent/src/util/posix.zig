const builtin = @import("builtin");
const std = @import("std");

const c = struct {
    extern "c" fn setenv(name: [*:0]const u8, value: [*:0]const u8, overwrite: c_int) c_int;
    extern "c" fn unsetenv(name: [*:0]const u8) c_int;
};

fn assertValidKey(key: [*:0]const u8) void {
    if (builtin.mode == .Debug or builtin.mode == .ReleaseSafe) {
        var ptr = key;
        if (ptr[0] == 0) {
            std.debug.panic("Zero-length environment variable key: {s}", .{key});
        }
        while (true) : (ptr += 1) {
            switch (ptr[0]) {
                '=' => std.debug.panic("Embedded '=' in environment variable key: {s}", .{key}),
                0 => break,
                else => {},
            }
        }
    }
}

pub fn setenv(key: [*:0]const u8, value: [*:0]const u8, overwrite: bool) !void {
    assertValidKey(key);
    const rc = c.setenv(key, value, @intFromBool(overwrite));
    switch (std.posix.errno(rc)) {
        .SUCCESS => {},
        .INVAL => unreachable, // key has been validated
        .NOMEM => return error.OutOfMemory,
        else => |err| std.debug.panic("unexpected errno: {d}\n", .{@intFromEnum(err)}),
    }
}

pub fn unsetenv(key: [*:0]const u8) !void {
    assertValidKey(key);
    const rc = c.unsetenv(key);
    switch (std.posix.errno(rc)) {
        .SUCCESS => {},
        // unclear if this is even a valid error from unsetenv
        .INVAL => unreachable, // key has been validated
        // unclear if this is a valid error from unsetenv. Doesn't make sense to be.
        .NOMEM => return error.OutOfMemory,
        else => |err| std.debug.panic("unexpected errno: {d}\n", .{@intFromEnum(err)}),
    }
}
