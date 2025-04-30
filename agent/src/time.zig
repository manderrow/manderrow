const std = @import("std");

pub const Month = enum(u4) {
    jan = 1,
    feb,
    mar,
    apr,
    may,
    jun,
    jul,
    aug,
    sep,
    oct,
    nov,
    dec,
};

/// Unix timestamp.
pub const Timestamp = packed struct {
    /// Range is restricted to that supported by `decodeUnixTime`.
    year: u32,
    month: Month,
    /// [1, 356]
    day: u9,
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
    var day_n = day_n_epoch + time / std.time.s_per_day;
    const seconds_since_midnight: u17 = @intCast(time % std.time.s_per_day);

    // Gregorian calendar leap year rule: if divisible by 100, only if divisible by 400
    var temp = 4 * (day_n + days_per_century + 1) / days_per_4_centuries - 1;
    var year = 100 * temp;
    day_n -= temp * days_per_century + temp / 4;

    // Julian calendar leap year rule: if divisible by 4
    temp = (4 * (day_n + days_per_year + 1)) / days_per_4_years - 1;
    year += temp;
    day_n -= temp * days_per_year + temp / 4;

    // see the original for a breakdown of these
    var month = (5 * day_n + 2) / 153;
    const day = day_n - (month * 153 + 2) / 5 + 1;

    // adjust month and year from previous-March-based to January-based
    month += 3;
    if (month > 12) {
        @branchHint(.unpredictable);
        month -= 12;
        year += 1;
    }

    return .{
        .year = @intCast(year),
        .month = @enumFromInt(month),
        .day = @intCast(day),
        .hour = @intCast(seconds_since_midnight / std.time.s_per_hour),
        .minute = @intCast(seconds_since_midnight % std.time.s_per_hour / std.time.s_per_min),
        .second = @intCast(seconds_since_midnight % std.time.s_per_min),
    };
}

test "decodeUnixTime" {
    const time = 1745980904;
    const ts: Timestamp = .{
        .year = 2025,
        .month = .apr,
        .day = 30,
        .hour = 2,
        .minute = 41,
        .second = 44,
    };
    try std.testing.expectEqual(ts, decodeUnixTime(time));
}
