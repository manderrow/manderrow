const std = @import("std");

const epoch = std.time.epoch;

/// Unix timestamp.
pub const Timestamp = packed struct {
    /// Range is restricted to that supported by `decodeUnixTime`.
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

/// Length of regular year.
const days_per_year = 365;
const days_per_4_years = 4 * 365 + 1;
const days_per_century = 100 * 365 + 25 - 1;
const days_per_4_centuries = 400 * 365 + 100 - 4 + 1;
/// Magic day number of the epoch (but really of the previous? March).
const day_n_epoch = 719468;

/// This is based on the implementation found [here](https://de.wikipedia.org/wiki/Unixzeit#Beispiel-Implementierung).
///
/// This function is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/deed.en).
///
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
