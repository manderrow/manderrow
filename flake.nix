{
  description = "Manderrow";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, rust-overlay, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];

      forAllSystems = f:
        nixpkgs.lib.genAttrs systems (system: f system);

      # Cargo settings for Nix builds only, as environment variables override .cargo/config.toml.
      cargoEnv = pkgs: {
        # Links with mold rather than lld. The flag must be in RUSTFLAGS for qt-build-utils to see it.
        "CARGO_TARGET_${pkgs.stdenv.hostPlatform.rust.cargoEnvVarTarget}_RUSTFLAGS" =
          "-Clink-arg=-fuse-ld=mold";
        # sccache cannot cache incremental builds
        CARGO_INCREMENTAL = "0";
      };
    in
    {
      packages = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain =
            pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          rustPlatform = pkgs.makeRustPlatform {
            cargo = rustToolchain;
            rustc = rustToolchain;
          };

          qtEnv = pkgs.qt6.env "qt6" (with pkgs.qt6; [
            qtbase
            qtdeclarative
            qtshadertools
            qttools
            qtimageformats
            qtsvg
          ]);

          agentZigDeps = pkgs.zig_0_17.fetchDeps {
            pname = "manderrow-agent";
            version = "0.0.0";
            src = ./agent;
            fetchAll = true;
            hash = pkgs.lib.fakeHash;
          };
        in
        {
          default = rustPlatform.buildRustPackage (cargoEnv pkgs // {
            pname = "manderrow";
            version = "0.1.0";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "ipc-channel-0.20.1" = pkgs.lib.fakeHash;
                "rkyv_intern-0.1.0" = "sha256-Y5lgD40YlsesGKHbS8k45OvJ4tOaCwRfuuo9v2EOlHU=";
                "get-locale-0.1.0" = "sha256-m0YM3BA5ttmzqTva4664uVrI16CTAjZTZ6gQLia7XVU=";
                "vdf-0.1.0" = "sha256-NaBEm2pPOAAQ1+zN6gtBHjlmv8fCrKaD9CzoVbyoq2w=";
              };
            };

            nativeBuildInputs = [
              rustToolchain
              pkgs.pkg-config
              pkgs.zig_0_17
              qtEnv
              pkgs.wine64
              pkgs.sccache
              pkgs.mold
              pkgs.qt6.wrapQtAppsHook
            ];

            buildInputs = [
              qtEnv
              pkgs.qt6.qtbase
            ];

            # qtbase's overrides QMAKE so it can't find the other Qt modules
            # so we have to override it again
            preBuild = ''
              export QMAKE="${qtEnv}/bin/qmake"

              export ZIG_GLOBAL_CACHE_DIR="$PWD/zig-global-cache"
              export ZIG_LOCAL_CACHE_DIR="$PWD/zig-local-cache"
              mkdir -p "$ZIG_GLOBAL_CACHE_DIR"
              cp -r ${agentZigDeps} "$ZIG_GLOBAL_CACHE_DIR/p"
              chmod -R u+w "$ZIG_GLOBAL_CACHE_DIR/p"
            '';

            XDG_CACHE_HOME = "xdg-cache";

            # zig tries to replace cargo
            dontUseZigConfigure = true;
            dontUseZigBuild = true;
            dontUseZigCheck = true;
            dontUseZigInstall = true;

            auditable = false;
            doCheck = false;

            meta = with pkgs.lib; {
              description = "Manderrow";
              mainProgram = "manderrow";
              platforms = platforms.linux;
            };
          });
        });

      devShells = forAllSystems (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain =
            pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

          qtEnv = pkgs.qt6.env "qt6" (with pkgs.qt6; [
            qtbase
            qtdeclarative
            qtshadertools
            qttools
            qtimageformats
            qtsvg
          ]);
        in
        {
          default = pkgs.mkShell (cargoEnv pkgs // {
            inputsFrom = [ self.packages.${system}.default ];

            packages = [
              rustToolchain
              pkgs.rust-analyzer
              pkgs.pkg-config
              pkgs.zig_0_17
              qtEnv
              pkgs.wine64
              pkgs.sccache
              pkgs.mold
            ];

            shellHook = ''
              export QMAKE="${qtEnv}/bin/qmake"
              export RUSTC_WRAPPER="${pkgs.sccache}/bin/sccache"
            '';
          });
        });
    };
}
