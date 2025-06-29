const std = @import("std");

const IpcMode = enum {
    ipc_channel,
    stderr,
    winelib,
};

pub fn build(b: *std.Build) !void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{
        .preferred_optimize_mode = .ReleaseSafe,
    });

    const strip = b.option(bool, "strip", "Forces stripping on all optimization modes") orelse switch (optimize) {
        .Debug => false,
        .ReleaseSafe, .ReleaseFast, .ReleaseSmall => true,
    };

    const ipc = b.option(IpcMode, "ipc-mode", "Determines the IPC mechanism used") orelse .ipc_channel;
    const wine = b.option(bool, "wine", "Compiles ipc-channel with the unix-on-wine feature") orelse false;
    const host_lib = b.option(bool, "host-lib", "Compiles in host library mode. The entrypoint will not be exported.") orelse false;

    const build_zig_zon = b.createModule(.{
        .root_source_file = b.path("build.zig.zon"),
    });

    {
        const lib = try createLib(b, target, optimize, strip, wine, ipc, host_lib, build_zig_zon, b.getInstallStep());

        b.getInstallStep().dependOn(&b.addInstallArtifact(lib.compile, .{
            .dest_dir = .{ .override = .lib },
        }).step);

        const lib_unit_tests = b.addTest(.{
            .root_module = lib.mod,
        });

        lib_unit_tests.linkSystemLibrary("unwind");

        const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);

        const test_step = b.step("test", "Run unit tests");
        test_step.dependOn(&run_lib_unit_tests.step);
    }

    const build_all_step = b.step("build-all", "Builds for all supported targets");

    const Cfg = struct {
        target: std.Build.ResolvedTarget,
        wine: bool = false,
        ipc: IpcMode = .ipc_channel,
        host_lib: bool = false,
    };

    inline for ([_]Cfg{
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .linux, .abi = .gnu }) },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .linux, .abi = .gnu }), .host_lib = true },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .macos }) },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .windows, .abi = .gnu }) },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .windows, .abi = .gnu }), .wine = true },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .windows, .abi = .gnu }), .ipc = .stderr },
        .{ .target = b.resolveTargetQuery(.{ .cpu_arch = .x86_64, .os_tag = .windows, .abi = .gnu }), .ipc = .winelib },
    }) |cfg| {
        const lib_2 = try createLib(b, cfg.target, optimize, strip, cfg.wine, cfg.ipc, cfg.host_lib, build_zig_zon, build_all_step);
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
    wine: bool,
    ipc: IpcMode,
    host_lib: bool,
    build_zig_zon: *std.Build.Module,
    install_step: *std.Build.Step,
) !struct { mod: *std.Build.Module, compile: *std.Build.Step.Compile } {
    const lib_mod = b.createModule(.{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = if (target.result.os.tag == .windows and optimize == .Debug) .Debug else optimize,
        .strip = strip,
        .link_libc = true,
    });
    if (target.result.os.tag == .windows and target.result.abi.isGnu()) {
        for ([_][]const u8{
            "api-ms-win-core-com-l1-1-0",
            "api-ms-win-core-handle-l1-1-0",
            "api-ms-win-core-winrt-error-l1-1-0",
            "api-ms-win-downlevel-ole32-l1-1-0",
            "oleaut32",
            "propsys",
            "unwind",
            "userenv",
            "ws2_32",
        }) |lib_name| {
            lib_mod.linkSystemLibrary(lib_name, .{});
        }
    }

    lib_mod.addImport("build.zig.zon", build_zig_zon);
    const options = b.addOptions();
    options.addOption(IpcMode, "ipc_mode", ipc);
    options.addOption(bool, "host_lib", host_lib);
    lib_mod.addOptions("build_options", options);

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

    const lib = b.addLibrary(.{
        .linkage = .dynamic,
        .name = "manderrow_agent",
        .root_module = lib_mod,
        // TODO: remove when possible ("Unimplemented: ExportOptions.section")
        .use_llvm = true,
    });

    const dep = b.dependency("wine_host_dlfcn", .{ .optimize = optimize });

    if (host_lib) {
        install_step.dependOn(&b.addInstallLibFile(dep.namedLazyPath("lib"), "host_dlfcn.dll.so").step);
    }

    switch (ipc) {
        .ipc_channel => {
            const cargo_build = b.addSystemCommand(&.{
                "cargo",
                "build",
                "--package",
                "manderrow-agent",
                "--profile",
                switch (optimize) {
                    .Debug => "dev",
                    .ReleaseSafe, .ReleaseFast, .ReleaseSmall => "release",
                },
                "--target",
                rust_target,
                "--manifest-path",
            });
            cargo_build.addFileArg(b.path("../crates/Cargo.toml"));
            if (wine) {
                cargo_build.addArgs(&.{ "--features", "unix-on-wine" });
            }
            // cargo_build.addFileInput(b.path("../crates/Cargo.lock"));
            // cargo_build.addFileInput(b.path("../crates/args"));
            // cargo_build.addFileInput(b.path("../crates/ipc"));
            // cargo_build.addFileInput(b.path("../crates/agent"));
            cargo_build.has_side_effects = true;

            lib.step.dependOn(&cargo_build.step);
            lib.addObjectFile(b.path(b.fmt("../crates/target/{s}/{s}/{s}", .{
                rust_target,
                switch (optimize) {
                    .Debug => "debug",
                    .ReleaseSafe, .ReleaseFast, .ReleaseSmall => "release",
                },
                switch (target.result.abi) {
                    .msvc => "manderrow_agent_rs.lib",
                    else => "libmanderrow_agent_rs.a",
                },
            })));
        },
        .winelib => {
            lib_mod.addImport("dlfcn", dep.module("proxy"));
        },
        .stderr => {},
    }

    return .{ .mod = lib_mod, .compile = lib };
}
