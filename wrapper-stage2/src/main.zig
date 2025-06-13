const std = @import("std");

const alloc = std.heap.smp_allocator;

pub fn main() !void {
    var args = try std.process.argsWithAllocator(alloc);
    if (!args.skip()) return error.MissingExeArg;

    var argv: std.ArrayListUnmanaged([]const u8) = .empty;

    while (args.next()) |arg| {
        try argv.append(alloc, arg);
    }

    var env: std.process.EnvMap = try std.process.getEnvMap(alloc);

    if (env.get("MANDERROW_WRAPPER_ENV")) |extra_env| {
        var scanner = std.json.Scanner.initCompleteInput(alloc, extra_env);
        switch (try scanner.next()) {
            .object_begin => {},
            else => return error.InvalidEnv,
        }
        while (true) {
            switch (try scanner.next()) {
                .object_end => break,
                .string, .allocated_string => |key| {
                    switch (try scanner.next()) {
                        .string, .allocated_string => |value| {
                            try env.put(key, value);
                        },
                        else => return error.InvalidEnv,
                    }
                },
                else => return error.InvalidEnv,
            }
        }
    }

    var child = std.process.Child.init(argv.items, alloc);
    child.env_map = &env;

    const term = try child.spawnAndWait();
    switch (term) {
        .Exited => |status| std.process.exit(status),
        else => {
            std.debug.print("Process exited with {}", .{term});
            std.process.exit(1);
        },
    }
}
