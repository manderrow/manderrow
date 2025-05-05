const builtin = @import("builtin");
const std = @import("std");
const Dir = std.fs.Dir;

const alloc = @import("root.zig").alloc;
const os_char = @import("util.zig").os_char;

pub var logs_dir: ?std.fs.Dir = null;
var init_logs_dir_lock: std.Thread.Mutex = .{};
pub var start_time: i64 = 0;

pub fn getOrInitLogsDir(override: ?Dir) Dir {
    if (!init_logs_dir_lock.tryLock()) {
        // We're crashing. Fallback to working directory so we can write *something*.
        return std.fs.cwd();
    }

    defer init_logs_dir_lock.unlock();
    if (logs_dir) |dir| {
        return dir;
    }
    logs_dir = override orelse defaultLogsDir() catch std.fs.cwd();
    return logs_dir.?;
}

const log_file_ts_fmt = "-{:0>4}-{:0>2}-{:0>2}T{:0>2}:{:0>2}:{:0>2}.{:0>3}Z.log";
const log_file_ts_len = std.fmt.count(log_file_ts_fmt, .{ 0, 0, 0, 0, 0, 0, 0 });

pub fn LogFileName(comptime label_utf8: [:0]const u8) type {
    return struct {
        pub const Data = [label.len + log_file_ts_len:0]os_char;

        const label = "manderrow-agent-" ++ osStrLiteral(label_utf8);
    };
}

pub fn logFileName(comptime label: [:0]const u8) LogFileName(label).Data {
    const T = LogFileName(label);
    var data: T.Data = undefined;
    @memcpy(data[0..T.label.len], T.label);
    // If your clock goes backwards, you have a problem.
    const time: u64 = std.math.cast(u64, start_time) orelse {
        @memcpy(data[T.label.len..], comptime osStrLiteral(std.fmt.comptimePrint(log_file_ts_fmt, .{ 0, 0, 0, 0, 0, 0, 0 })));
        return data;
    };
    const ms: u10 = @intCast(time % 1000);
    var ts = @import("time.zig").decodeUnixTime(@divTrunc(time, 1000));
    if (ts.year > 9999) {
        // not dealing with this.
        ts.year = 9999;
    }
    const args = .{ ts.year, @intFromEnum(ts.month), ts.day, ts.hour, ts.minute, ts.second, ms };
    switch (builtin.os.tag) {
        .windows => {
            var utf8_buf: [log_file_ts_len]u8 = undefined;
            const utf8_ts = std.fmt.bufPrint(&utf8_buf, log_file_ts_fmt, args) catch unreachable;
            const n = std.unicode.utf8ToUtf16Le(data[T.label.len..], utf8_ts) catch unreachable;
            std.debug.assert(n == data[T.label.len..].len);
        },
        else => {
            const slice = std.fmt.bufPrint(data[T.label.len..], log_file_ts_fmt, args) catch unreachable;
            std.debug.assert(data[T.label.len..].len == slice.len);
        },
    }
    return data;
}

/// This will create the directory if it does not exist.
fn defaultLogsDir() !std.fs.Dir {
    var app_data_local_dir = try appDataLocalDir();
    defer app_data_local_dir.close();

    return openOrCreateDir(app_data_local_dir, osStrLiteral("logs"));
}

/// This will create the directory if it does not exist.
///
/// This function matches the behaviour of our Rust paths crate.
fn appDataLocalDir() !std.fs.Dir {
    var data_local_dir = try dataLocalDir();
    defer data_local_dir.close();

    return openOrCreateDir(data_local_dir, osStrLiteral(switch (builtin.os.tag) {
        .macos, .windows => "Manderrow",
        else => "manderrow",
    }));
}

pub fn osStrLiteral(comptime path: [:0]const u8) [:0]const os_char {
    return comptime switch (builtin.os.tag) {
        .windows => std.unicode.utf8ToUtf16LeStringLiteral(path),
        else => path,
    };
}

pub fn openOrCreateDir(dir: Dir, sub_path: [:0]const os_char) !Dir {
    switch (builtin.os.tag) {
        .windows => {
            dir.makeDirW(sub_path) catch |e| switch (e) {
                error.PathAlreadyExists => {},
                else => return e,
            };
            return dir.openDirW(sub_path, .{});
        },
        else => {
            dir.makeDirZ(sub_path) catch |e| switch (e) {
                error.PathAlreadyExists => {},
                else => return e,
            };
            return dir.openDirZ(sub_path, .{});
        },
    }
}

// the following functions match the behaviour of the dirs crate version 6.0.0.

fn dataLocalDir() !std.fs.Dir {
    switch (builtin.os.tag) {
        .linux => {
            if (std.process.getEnvVarOwned(alloc, "XDG_DATA_HOME") catch |e| switch (e) {
                error.EnvironmentVariableNotFound => null,
                else => return e,
            }) |path| {
                defer alloc.free(path);
                if (path.len != 0) {
                    return std.fs.openDirAbsolute(path, .{});
                }
            }

            var home_dir = try homeDir();
            defer home_dir.close();
            return home_dir.openDirZ(".local/share", .{});
        },
        .windows => {
            return SHGetKnownFolder(std.os.windows.FOLDERID_LocalAppData);
        },
        .macos => {
            var home_dir = try homeDir();
            defer home_dir.close();
            return home_dir.openDirZ("Library/Application Support", .{});
        },
        else => |os| @compileError("Unsupported OS: " ++ @tagName(os)),
    }
}

fn homeDir() !std.fs.Dir {
    if (std.process.getEnvVarOwned(alloc, "HOME") catch |e| switch (e) {
        error.EnvironmentVariableNotFound => null,
        else => return e,
    }) |path| {
        defer alloc.free(path);
        if (path.len != 0) {
            return std.fs.openDirAbsolute(path, .{});
        }
    }

    const _SC_GETPW_R_SIZE_MAX = 70;
    const amt = std.math.cast(usize, std.c.sysconf(_SC_GETPW_R_SIZE_MAX)) orelse 512;
    const buf = try alloc.alloc(u8, amt);
    defer alloc.free(buf);
    var passwd = std.mem.zeroes(std.c.passwd);
    var result: ?*std.c.passwd = null;
    switch (std.posix.errno(getpwuid_r(
        std.posix.getuid(),
        &passwd,
        buf.ptr,
        buf.len,
        &result,
    ))) {
        .SUCCESS => {
            if (result) |_| {
                if (passwd.dir) |dir| {
                    if (dir[0] != 0) {
                        return std.fs.openDirAbsoluteZ(dir, .{});
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

fn SHGetKnownFolder(fid: std.os.windows.KNOWNFOLDERID) !Dir {
    // store this on the heap to minimize stack usage.
    const buf = try std.heap.smp_allocator.create([std.os.windows.PATH_MAX_WIDE:0]u16);
    defer std.heap.smp_allocator.destroy(buf);
    switch (std.os.windows.HRESULT_CODE(SHGetFolderPathEx(&fid, 0, null, buf, buf.len))) {
        .SUCCESS => {},
        else => |e| return std.os.windows.unexpectedError(e),
    }

    const prefixed_path = try std.os.windows.wToPrefixedFileW(null, std.mem.span(@as([*:0]u16, buf)));
    return std.fs.openDirAbsoluteW(prefixed_path.span(), .{});
}

// this is missing from the Zig std.c bindings on Linux
pub extern "c" fn getpwuid_r(uid: std.posix.uid_t, pw: *std.c.passwd, buf: [*]u8, buflen: usize, pwretp: *?*std.c.passwd) c_int;

pub extern "api-ms-win-core-com-l1-1-0" fn SHGetFolderPathEx(
    rfid: *const std.os.windows.KNOWNFOLDERID,
    dwFlags: std.os.windows.DWORD,
    hToken: ?std.os.windows.HANDLE,
    pszPath: std.os.windows.PWSTR,
    cchPath: std.os.windows.UINT,
) std.os.windows.HRESULT;

pub extern "wsmsvc" fn CoTaskMemFree(ptr: ?*anyopaque) void;

test "dataLocalDir" {
    var dir = try dataLocalDir();
    dir.close();
}

test "appDataLocalDir" {
    var dir = try appDataLocalDir();
    dir.close();
}

test "defaultLogsDir" {
    var dir = try defaultLogsDir();
    dir.close();
}

test "logFileName" {
    start_time = 1745980904000;
    const expected = "manderrow-agent-crash-2025-04-30T02:41:44.000Z.log";
    const actual = logFileName("crash");
    switch (builtin.os.tag) {
        .windows => try std.testing.expectEqualSlices(u16, std.unicode.utf8ToUtf16LeStringLiteral(expected), &actual),
        else => try std.testing.expectEqualStrings(expected, &actual),
    }
}
