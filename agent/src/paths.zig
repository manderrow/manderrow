const builtin = @import("builtin");
const std = @import("std");
const Dir = std.Io.Dir;

const root = @import("root.zig");
const alloc = root.alloc;
const io = root.io;

pub var logs_dir: ?Dir = null;
var init_logs_dir_lock: std.Io.Mutex = .init;
pub var start_time: i64 = 0;

pub fn getOrInitLogsDir(override: ?Dir) Dir {
    if (!init_logs_dir_lock.tryLock()) {
        // We're crashing. Fallback to working directory so we can write *something*.
        return Dir.cwd();
    }

    defer init_logs_dir_lock.unlock(io);
    if (logs_dir) |dir| {
        return dir;
    }
    logs_dir = override orelse defaultLogsDir() catch Dir.cwd();
    return logs_dir.?;
}

const log_file_ts_fmt = "-{:0>4}-{:0>2}-{:0>2}T{:0>2}-{:0>2}-{:0>2}.{:0>3}Z.log";
const log_file_ts_len = std.fmt.count(log_file_ts_fmt, .{ 0, 0, 0, 0, 0, 0, 0 });

pub fn LogFileName(comptime label_utf8: [:0]const u8) type {
    return struct {
        pub const Data = [label.len + log_file_ts_len:0]u8;

        const label = "manderrow-agent-" ++ label_utf8;
    };
}

pub fn logFileName(comptime label: [:0]const u8) LogFileName(label).Data {
    const T = LogFileName(label);
    var data: T.Data = undefined;
    @memcpy(data[0..T.label.len], T.label);
    // If your clock goes backwards, you have a problem.
    const time: u64 = std.math.cast(u64, start_time) orelse {
        @memcpy(data[T.label.len..], comptime std.fmt.comptimePrint(log_file_ts_fmt, .{ 0, 0, 0, 0, 0, 0, 0 }));
        return data;
    };
    const ms: u10 = @intCast(time % 1000);
    var ts = @import("time.zig").decodeUnixTime(@divTrunc(time, 1000));
    if (ts.year > 9999) {
        // not dealing with this.
        ts.year = 9999;
    }
    const args = .{ ts.year, @backingInt(ts.month), @as(u6, ts.day_index) + 1, ts.hour, ts.minute, ts.second, ms };
    const slice = std.fmt.bufPrint(data[T.label.len..], log_file_ts_fmt, args) catch unreachable;
    std.debug.assert(data[T.label.len..].len == slice.len);
    data[data.len] = 0;
    return data;
}

/// This will create the directory if it does not exist.
fn defaultLogsDir() !Dir {
    var app_data_local_dir = try appDataLocalDir();
    defer app_data_local_dir.close(io);

    return openOrCreateDir(app_data_local_dir, "logs");
}

/// This will create the directory if it does not exist.
///
/// This function matches the behaviour of our Rust paths crate.
fn appDataLocalDir() !Dir {
    var data_local_dir = try dataLocalDir();
    defer data_local_dir.close(io);

    return openOrCreateDir(data_local_dir, switch (builtin.target.os.tag) {
        .macos, .windows => "Manderrow",
        else => "manderrow",
    });
}

pub fn openOrCreateDir(dir: Dir, sub_path: []const u8) !Dir {
    return dir.createDirPathOpen(io, sub_path, .{});
}

// the following functions match the behaviour of the dirs crate version 6.0.0.

fn dataLocalDir() !Dir {
    switch (builtin.target.os.tag) {
        .linux => {
            if (root.processEnviron().getPosix("XDG_DATA_HOME")) |path| {
                if (path.len != 0) {
                    return Dir.openDirAbsolute(io, path, .{});
                }
            }

            var home_dir = try homeDir();
            defer home_dir.close(io);
            return home_dir.openDir(io, ".local/share", .{});
        },
        .windows => {
            return SHGetKnownFolder(&FOLDERID_LocalAppData);
        },
        .macos => {
            var home_dir = try homeDir();
            defer home_dir.close(io);
            return home_dir.openDir(io, "Library/Application Support", .{});
        },
        else => |os| @compileError("Unsupported OS: " ++ @tagName(os)),
    }
}

fn homeDir() !Dir {
    if (root.processEnviron().getPosix("HOME")) |path| {
        if (path.len != 0) {
            return Dir.openDirAbsolute(io, path, .{});
        }
    }

    const _SC_GETPW_R_SIZE_MAX = 70;
    const amt = std.math.cast(usize, std.c.sysconf(_SC_GETPW_R_SIZE_MAX)) orelse 512;
    const buf = try alloc.alloc(u8, amt);
    defer alloc.free(buf);
    var passwd = std.mem.zeroes(std.c.passwd);
    var result: ?*std.c.passwd = null;
    switch (std.posix.errno(getpwuid_r(
        std.c.getuid(),
        &passwd,
        buf.ptr,
        buf.len,
        &result,
    ))) {
        .SUCCESS => {
            if (result) |_| {
                if (passwd.dir) |dir| {
                    if (dir[0] != 0) {
                        return Dir.openDirAbsolute(io, std.mem.span(dir), .{});
                    }
                }
            }
            return error.NotFound;
        },
        .NOENT, .SRCH, .BADF, .PERM => return error.NotFound,
        .INTR => return error.Interrupted,
        .IO => return error.InputOutput,
        .MFILE => return error.ProcessFdQuotaExceeded,
        .NFILE => return error.SystemFdQuotaExceeded,
        .NOMEM => return error.SystemResources,
        .RANGE => return error.NameTooLong,
        else => |e| return std.posix.unexpectedErrno(e),
    }
}

fn SHGetKnownFolder(fid: *const KNOWNFOLDERID) !Dir {
    // store this on the heap to minimize stack usage.
    const buf = try alloc.create([std.os.windows.PATH_MAX_WIDE:0]u16);
    defer alloc.destroy(buf);
    const hr = SHGetFolderPathEx(fid, 0, null, buf, buf.len);
    if (hr != S_OK) {
        // the facility is FACILITY_WIN32 for every error that SHGetFolderPathEx documents
        return std.os.windows.unexpectedError(@fromBackingInt(@intCast(hr & 0xFFFF)));
    }

    const path = try std.unicode.wtf16LeToWtf8Alloc(alloc, std.mem.span(@as([*:0]u16, buf)));
    defer alloc.free(path);
    return Dir.openDirAbsolute(io, path, .{});
}

// the standard library no longer binds the shell
const HRESULT = c_long;
const S_OK: HRESULT = 0;
const KNOWNFOLDERID = std.os.windows.GUID;
const FOLDERID_LocalAppData: KNOWNFOLDERID = .parse("{F1B32785-6FBA-4FCF-9D55-7B8E7F157091}");

// this is missing from the Zig std.c bindings on Linux
pub extern "c" fn getpwuid_r(uid: std.posix.uid_t, pw: *std.c.passwd, buf: [*]u8, buflen: usize, pwretp: *?*std.c.passwd) c_int;

pub extern "api-ms-win-core-com-l1-1-0" fn SHGetFolderPathEx(
    rfid: *const KNOWNFOLDERID,
    dwFlags: std.os.windows.DWORD,
    hToken: ?std.os.windows.HANDLE,
    pszPath: std.os.windows.PWSTR,
    cchPath: std.os.windows.UINT,
) HRESULT;

pub extern "wsmsvc" fn CoTaskMemFree(ptr: ?*anyopaque) void;

test "dataLocalDir" {
    var dir = try dataLocalDir();
    dir.close(io);
}

test "appDataLocalDir" {
    var dir = try appDataLocalDir();
    dir.close(io);
}

test "defaultLogsDir" {
    var dir = try defaultLogsDir();
    dir.close(io);
}

test "logFileName" {
    start_time = 1745980904000;
    const expected = "manderrow-agent-crash-2025-04-30T02-41-44.000Z.log";
    const actual = logFileName("crash");
    try std.testing.expectEqualStrings(expected, &actual);
}
