set windows-powershell

export RUST_BACKTRACE := "1"
export RUST_LOG := "debug"

run *ARGS:
	pnpm tauri dev {{ARGS}}

build *ARGS:
	pnpm tauri build {{ARGS}}

fmt:
	cargo fmt --manifest-path crates/Cargo.toml --all
	cargo fmt --manifest-path src-tauri/Cargo.toml
	zig fmt agent

clean:
	cargo clean --manifest-path crates/Cargo.toml
	cargo clean --manifest-path src-tauri/Cargo.toml
	rm -vrf agent/.zig-cache
	rm -vrf agent/zig-out

test:
	cargo test --manifest-path crates/Cargo.toml
	cargo test --manifest-path src-tauri/Cargo.toml

update:
	cargo update --manifest-path crates/Cargo.toml --verbose
	cargo update --manifest-path src-tauri/Cargo.toml --verbose
	pnpm update

check:
	cargo check --manifest-path crates/Cargo.toml
	cargo check --manifest-path src-tauri/Cargo.toml

run-script BIN *ARGS:
	cargo run --manifest-path crates/Cargo.toml --bin {{BIN}} -- {{ARGS}}

[working-directory: 'scripts/game-updater']
add-games:
	deno run add-games

[working-directory: 'scripts/game-updater']
scrape:
	deno run scrape

