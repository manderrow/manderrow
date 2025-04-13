const std = @import("std");

pub fn build(b: *std.Build) !void {
    const target = b.standardTargetOptions(.{});

    const optimize = b.standardOptimizeOption(.{});

    const strip = b.option(bool, "strip", "Forces stripping on all optimization modes") orelse switch (optimize) {
        .Debug => false,
        .ReleaseSafe, .ReleaseFast, .ReleaseSmall => true,
    };

    {
        const lib = try createLib(b, target, optimize, strip);

        b.getInstallStep().dependOn(&b.addInstallArtifact(lib.compile, .{
            .dest_dir = .{ .override = .lib },
        }).step);

        const lib_unit_tests = b.addTest(.{
            .root_module = lib.mod,
        });

        const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);

        const test_step = b.step("test", "Run unit tests");
        test_step.dependOn(&run_lib_unit_tests.step);
    }

    const build_all_step = b.step("build-all", "Builds for all supported targets");

    inline for ([_]std.Build.ResolvedTarget{
        b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .linux, .abi = .gnu }),
        b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .macos }),
        b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .windows, .abi = .gnu }),
    }) |target_2| {
        const lib_2 = try createLib(b, target_2, optimize, strip);
        build_all_step.dependOn(&b.addInstallArtifact(lib_2.compile, .{
            .dest_dir = .{ .override = .lib },
        }).step);
    }
}

fn createLib(
    b: *std.Build,
    target: std.Build.ResolvedTarget,
    optimize: std.builtin.OptimizeMode,
    strip: bool,
) !struct { mod: *std.Build.Module, compile: *std.Build.Step.Compile } {
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    if (target.result.os.tag == .windows and target.result.abi.isGnu()) {
        for ([_][]const u8{
            "api-ms-win-core-com-l1-1-0",
            "api-ms-win-core-handle-l1-1-0",
            "api-ms-win-core-winrt-error-l1-1-0",
            "api-ms-win-downlevel-ole32-l1-1-0",
            "oleaut32",
            "unwind",
            "userenv",
            "ws2_32",
        }) |lib_name| {
            lib_mod.linkSystemLibrary(lib_name, .{});
        }
    }

    const dll_proxy_dep = b.dependency("dll_proxy", .{
        .target = target,
        .optimize = optimize,
        .strip = strip,
    });

    lib_mod.addImport("dll_proxy", dll_proxy_dep.module("dll_proxy"));

    const rust_target = b.fmt("{s}-{s}-{s}{s}{s}", .{
        @tagName(target.result.cpu.arch),
        switch (target.result.os.tag) {
            .macos => "apple",
            .windows => "pc",
            .linux => "unknown",
            else => return error.UnsupportedOS,
        },
        switch (target.result.os.tag) {
            .macos => "darwin",
            else => |t| @tagName(t),
        },
        switch (target.result.os.tag) {
            .macos => "",
            else => "-",
        },
        switch (target.result.os.tag) {
            .macos => "",
            // .windows => "gnu",
            else => @tagName(target.result.abi),
        },
    });

    const cargo_build = b.addSystemCommand(&.{
        "cargo",
        "build",
        "--package",
        "manderrow-agent",
        "--release",
        "--target",
        rust_target,
        "--manifest-path",
    });
    cargo_build.addFileArg(b.path("../crates/Cargo.toml"));
    // cargo_build.addFileInput(b.path("../crates/Cargo.lock"));
    // cargo_build.addFileInput(b.path("../crates/args"));
    // cargo_build.addFileInput(b.path("../crates/ipc"));
    // cargo_build.addFileInput(b.path("../crates/agent"));
    cargo_build.has_side_effects = true;

    const lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "manderrow_agent",
        .root_module = lib_mod,
    });

    lib.step.dependOn(&cargo_build.step);
    lib.addObjectFile(b.path(b.fmt("../crates/target/{s}/release/{s}", .{
        rust_target, switch (target.result.abi) {
            .msvc => "manderrow_agent_rs.lib",
            else => "libmanderrow_agent_rs.a",
        },
    })));

    return .{ .mod = lib_mod, .compile = lib };
}
