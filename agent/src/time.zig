const std = @import("std");

const epoch = std.time.epoch;

/// Unix timestamp.
pub const Timestamp = packed struct {
    year: u16,
    month: epoch.Month,
    /// [0, 31]
    day_index: u5,
    /// [0, 23]
    hour: u5,
    /// [0, 59]
    minute: u6,
    /// [0, 59]
    second: u6,
};

/// - *time* is measured in seconds.
pub fn decodeUnixTime(time: u64) Timestamp {
    const epoch_secs: epoch.EpochSeconds = .{ .secs = time };
    const yd = epoch_secs.getEpochDay().calculateYearDay();
    const md = yd.calculateMonthDay();
    const ds = epoch_secs.getDaySeconds();
    return .{
        .year = yd.year,
        .month = md.month,
        .day_index = md.day_index,
        .hour = ds.getHoursIntoDay(),
        .minute = ds.getMinutesIntoHour(),
        .second = ds.getSecondsIntoMinute(),
    };
}

test "decodeUnixTime" {
    const time = 1745980904;
    const ts: Timestamp = .{
        .year = 2025,
        .month = .apr,
        .day_index = 30 - 1,
        .hour = 2,
        .minute = 41,
        .second = 44,
    };
    try std.testing.expectEqual(ts, decodeUnixTime(time));
}
