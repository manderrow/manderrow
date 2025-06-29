pub const LogLevel = enum(u8) {
    critical,
    err,
    warn,
    info,
    debug,
    trace,
};

pub const StandardOutputChannel = enum(u8) {
    out,
    err,
};
