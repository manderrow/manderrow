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
        in
        {
          default = rustPlatform.buildRustPackage {
            pname = "manderrow";
            version = "0.1.0";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "ipc-channel-0.20.1" = "sha256-2EPEAKMXlPhXZkmF06SKgxnI2EXDojrN6zJpy0s1AOc=";
                "rkyv_intern-0.1.0" = "sha256-Y5lgD40YlsesGKHbS8k45OvJ4tOaCwRfuuo9v2EOlHU=";
                "get-locale-0.1.0" = "sha256-m0YM3BA5ttmzqTva4664uVrI16CTAjZTZ6gQLia7XVU=";
                "vdf-0.1.0" = "sha256-NaBEm2pPOAAQ1+zN6gtBHjlmv8fCrKaD9CzoVbyoq2w=";
              };
            };

            nativeBuildInputs = [
              rustToolchain
              pkgs.pkg-config
              pkgs.zig_0_15
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
            '';

            XDG_CACHE_HOME = "xdg-cache";
            ZIG_GLOBAL_CACHE_DIR = "zig-global-cache";
            ZIG_LOCAL_CACHE_DIR = "zig-local-cache";

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
          };
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
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.default ];

            packages = [
              rustToolchain
              pkgs.rust-analyzer
              pkgs.pkg-config
              pkgs.zig_0_15
              qtEnv
              pkgs.wine64
              pkgs.sccache
              pkgs.mold
            ];

            shellHook = ''
              export QMAKE="${qtEnv}/bin/qmake"
              export RUSTC_WRAPPER="${pkgs.sccache}/bin/sccache"
            '';
          };
        });
    };
}
