const std = @import("std");

pub fn panicWindowsError(func: []const u8) noreturn {
    @branchHint(.cold);
    const err = std.os.windows.GetLastError();
    // 614 is the length of the longest windows error description
    var buf_wstr: [614:0]std.os.windows.WCHAR = undefined;
    const len = std.os.windows.kernel32.FormatMessageW(
        std.os.windows.FORMAT_MESSAGE_FROM_SYSTEM | std.os.windows.FORMAT_MESSAGE_IGNORE_INSERTS,
        null,
        err,
        (std.os.windows.SUBLANG.DEFAULT << 10) | std.os.windows.LANG.NEUTRAL,
        &buf_wstr,
        buf_wstr.len,
        null,
    );
    std.debug.panic("error.Unexpected(0x{x}): {s}: {}\n", .{
        @intFromEnum(err),
        func,
        std.unicode.fmtUtf16Le(buf_wstr[0..len]),
    });
}

pub fn SetEnvironmentVariable(key: [*:0]const u16, value: ?[*:0]const u16) void {
    if (std.os.windows.kernel32.SetEnvironmentVariableW(key, value) == 0) {
        panicWindowsError("SetEnvironmentVariableW");
    }
}
