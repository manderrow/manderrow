const std = @import("std");
const windows = std.os.windows;

const crash = @import("../crash.zig");

// the standard library no longer binds kernel32
pub const kernel32 = struct {
    pub extern "kernel32" fn LoadLibraryW(lpLibFileName: [*:0]const u16) callconv(.winapi) ?windows.HMODULE;

    pub extern "kernel32" fn SetEnvironmentVariableW(lpName: [*:0]const u16, lpValue: ?[*:0]const u16) callconv(.winapi) windows.BOOL;

    pub extern "kernel32" fn CreatePipe(
        hReadPipe: *windows.HANDLE,
        hWritePipe: *windows.HANDLE,
        lpPipeAttributes: ?*const windows.SECURITY_ATTRIBUTES,
        nSize: windows.DWORD,
    ) callconv(.winapi) windows.BOOL;

    pub extern "kernel32" fn FormatMessageW(
        dwFlags: windows.DWORD,
        lpSource: ?*const anyopaque,
        dwMessageId: windows.Win32Error,
        dwLanguageId: windows.DWORD,
        lpBuffer: [*]u16,
        nSize: windows.DWORD,
        Arguments: ?*anyopaque,
    ) callconv(.winapi) windows.DWORD;

    pub const FORMAT_MESSAGE_FROM_SYSTEM = 0x00001000;
    pub const FORMAT_MESSAGE_IGNORE_INSERTS = 0x00000200;
};

pub fn panicWindowsError(src: std.lang.SourceLocation, func: []const u8) noreturn {
    @branchHint(.cold);
    const err = windows.GetLastError();
    // 614 is the length of the longest windows error description
    var buf_wstr: [614:0]windows.WCHAR = undefined;
    const len = kernel32.FormatMessageW(
        kernel32.FORMAT_MESSAGE_FROM_SYSTEM | kernel32.FORMAT_MESSAGE_IGNORE_INSERTS,
        null,
        err,
        (windows.SUBLANG.DEFAULT << 10) | windows.LANG.NEUTRAL,
        &buf_wstr,
        buf_wstr.len,
        null,
    );
    crash.crash(src, "error.Unexpected(0x{x}): {s}: {f}\n", .{
        @backingInt(err),
        func,
        std.unicode.fmtUtf16Le(buf_wstr[0..len]),
    });
}

pub fn SetEnvironmentVariable(key: [*:0]const u16, value: ?[*:0]const u16) void {
    if (!kernel32.SetEnvironmentVariableW(key, value).toBool()) {
        panicWindowsError(@src(), "SetEnvironmentVariableW");
    }
}
