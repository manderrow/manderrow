const builtin = @import("builtin");
const std = @import("std");

const alloc = @import("root.zig").alloc;
const openOrCreateDir = @import("paths.zig").openOrCreateDir;

log_to_file: bool,
logs_dir: ?std.fs.Dir,
c2s_tx: ?[:0]const u8,
instructions: []Instruction,
dlfcn_host_path: ?[:0]const u8,
agent_host_path: ?[:0]const u8,

/// Holds allocations. Not meant to be used directly.
args: std.process.ArgIterator,

pub fn extract() !@This() {
    var args = try std.process.argsWithAllocator(alloc);

    var enabled = false;
    var instructions: std.ArrayListUnmanaged(Instruction) = .empty;
    errdefer instructions.deinit(alloc);
    var c2s_tx: ?[:0]const u8 = null;
    var dlfcn_host_path: ?[:0]const u8 = null;
    var agent_host_path: ?[:0]const u8 = null;

    var log_to_file = false;
    var logs_dir: ?std.fs.Dir = null;

    var extracting = false;
    while (args.next()) |arg| {
        if (extracting) {
            const token = std.meta.stringToEnum(enum {
                @"{manderrow",
                @"manderrow}",
                @"--enable",
                @"--log-to-file",
                @"--logs-dir",
                @"--c2s-tx",
                @"--insn-load-library",
                @"--insn-set-var",
                @"--insn-prepend-arg",
                @"--insn-append-arg",
                @"--agent-path",
                @"--dlfcn-host-path",
                @"--agent-host-path",
            }, arg) orelse return error.UnexpectedArgument;
            switch (token) {
                .@"{manderrow" => return error.UnbalancedArgumentDelimiter,
                .@"manderrow}" => extracting = false,
                .@"--enable" => enabled = true,
                .@"--log-to-file" => log_to_file = true,
                .@"--logs-dir" => {
                    const path = args.next() orelse return error.MissingOptionValue;
                    var os_path = switch (builtin.os.tag) {
                        .windows => blk: {
                            const buf = try alloc.allocSentinel(u16, try std.unicode.calcWtf16LeLen(path), 0);
                            const n = try std.unicode.wtf8ToWtf16Le(buf, path);
                            std.debug.assert(n == buf.len);
                            break :blk buf;
                        },
                        else => path,
                    };
                    defer if (builtin.os.tag == .windows) {
                        alloc.free(os_path);
                    };
                    switch (builtin.os.tag) {
                        .windows => {
                            if (std.fs.path.isAbsoluteWindowsWTF16(os_path)) {
                                const new_path_tmp = try std.os.windows.wToPrefixedFileW(null, os_path);
                                const new_path = try alloc.dupeZ(u16, new_path_tmp.span());
                                alloc.free(os_path);
                                os_path = new_path;
                            }
                        },
                        else => {},
                    }
                    logs_dir = try @import("paths.zig").openOrCreateDir(std.fs.cwd(), os_path);
                },
                .@"--c2s-tx" => {
                    c2s_tx = args.next() orelse return error.MissingOptionValue;
                    if (!std.unicode.utf8ValidateSlice(c2s_tx.?)) {
                        return error.InvalidUtf8;
                    }
                },
                .@"--insn-load-library" => try instructions.append(alloc, .{ .load_library = .{
                    .path = args.next() orelse return error.MissingOptionValue,
                } }),
                .@"--insn-set-var" => {
                    const kv = args.next() orelse return error.MissingOptionValue;
                    const eq_sign = std.mem.indexOfScalar(u8, kv, '=') orelse return error.InvalidSetVarKV;
                    try instructions.append(alloc, .{ .set_var = .{
                        .kv = kv,
                        .eq_sign = eq_sign,
                    } });
                },
                .@"--insn-prepend-arg" => try instructions.append(alloc, .{ .prepend_arg = .{
                    .arg = args.next() orelse return error.MissingOptionValue,
                } }),
                .@"--insn-append-arg" => try instructions.append(alloc, .{ .append_arg = .{
                    .arg = args.next() orelse return error.MissingOptionValue,
                } }),
                .@"--agent-path" => {
                    // arg for wrapper. ignore.
                    if (!args.skip()) {
                        return error.MissingOptionValue;
                    }
                },
                .@"--dlfcn-host-path" => {
                    dlfcn_host_path = args.next() orelse return error.MissingOptionValue;
                    if (!std.unicode.wtf8ValidateSlice(dlfcn_host_path.?)) {
                        return error.InvalidWtf8;
                    }
                },
                .@"--agent-host-path" => {
                    agent_host_path = args.next() orelse return error.MissingOptionValue;
                },
            }
        } else {
            // NOTE: this will break if a user or game decides to use one of these delimiters as an argument
            const token = std.meta.stringToEnum(enum {
                @"{manderrow",
                @"manderrow}",
            }, arg) orelse continue;
            switch (token) {
                .@"{manderrow" => extracting = true,
                .@"manderrow}" => return error.UnbalancedArgumentDelimiter,
            }
        }
    }

    if (!enabled) {
        return error.Disabled;
    }

    return .{
        .log_to_file = log_to_file,
        .logs_dir = logs_dir,
        .c2s_tx = c2s_tx,
        .instructions = try instructions.toOwnedSlice(alloc),
        .dlfcn_host_path = dlfcn_host_path,
        .agent_host_path = agent_host_path,

        .args = args,
    };
}

pub fn deinit(self: *@This()) void {
    alloc.free(self.instructions);
    self.instructions = undefined;
    self.args.deinit();
}

pub const Instruction = union(enum) {
    load_library: struct { path: [:0]const u8 },
    set_var: struct { kv: [:0]const u8, eq_sign: usize },
    prepend_arg: struct { arg: [:0]const u8 },
    append_arg: struct { arg: [:0]const u8 },
};
