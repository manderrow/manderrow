const builtin = @import("builtin");
const std = @import("std");

const proto = @import("wine_unixlib_proto.zig");
const rs = @import("rs.zig");

export const __wine_unix_call_funcs = blk: {
    var funcs: [proto.proxy_exports.len]*const anyopaque = undefined;
    for (proto.proxy_exports, &funcs) |name, *func| {
        func.* = &@field(@This(), name);
    }
    break :blk funcs;
};

pub fn manderrow_agent_init(args: *proto.InitArgs) std.os.windows.NTSTATUS {
    args.status = rs.impl.manderrow_agent_init(args.c2s_tx_ptr, args.c2s_tx_len, args.error_buf);
    return .SUCCESS;
}

pub fn manderrow_agent_send_exit(args: *proto.SendExitArgs) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_exit(args.code, args.with_code);
    return .SUCCESS;
}

pub fn manderrow_agent_send_crash(args: *proto.SendCrashArgs) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_crash(args.msg_ptr, args.msg_len);
    return .SUCCESS;
}

pub fn manderrow_agent_send_output_line(args: *proto.SendOutputLineArgs) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_output_line(args.channel, args.line_ptr, args.line_len);
    return .SUCCESS;
}

/// `scope` must consist entirely of ASCII characters in the range `'!'..='~'`.
/// `msg` must consist entirely of UTF-8 characters.
pub fn manderrow_agent_send_log(args: *proto.SendLogArgs) std.os.windows.NTSTATUS {
    rs.impl.manderrow_agent_send_log(args.level, args.scope_ptr, args.scope_len, args.msg_ptr, args.msg_len);
    return .SUCCESS;
}
