const builtin = @import("builtin");
const std = @import("std");

comptime {
    if (builtin.os.tag != .windows) @compileError("Windows-only code cannot be accessed from " ++ @tagName(builtin.os.tag));
}

const FdwReason = enum(std.os.windows.DWORD) {
    PROCESS_DETACH = 0,
    PROCESS_ATTACH = 1,
    THREAD_ATTACH = 2,
    THREAD_DETACH = 3,
    _,
};

pub var initialFrameAddress: usize = 0;

// noinline to improve debugging
pub noinline fn DllMain(
    hInstDll: std.os.windows.HINSTANCE,
    fdwReasonRaw: u32,
    _: std.os.windows.LPVOID,
) std.os.windows.BOOL {
    const module: std.os.windows.HMODULE = @ptrCast(hInstDll);

    const fdwReason: FdwReason = @enumFromInt(fdwReasonRaw);

    if (fdwReason != .PROCESS_ATTACH) {
        return std.os.windows.TRUE;
    }

    initialFrameAddress = @frameAddress();

    @call(.never_inline, @import("root.zig").entrypoint, .{module});

    return std.os.windows.TRUE;
}
