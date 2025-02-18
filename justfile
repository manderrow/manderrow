run *ARGS:
	RUST_BACKTRACE=1 RUST_LOG=debug deno run tauri dev {{ARGS}}

fmt:
	cargo fmt --manifest-path src-tauri/Cargo.toml
	cargo fmt --manifest-path crates/Cargo.toml

clean:
	cargo clean --manifest-path src-tauri/Cargo.toml
	cargo clean --manifest-path crates/Cargo.toml

test:
	cargo test --manifest-path src-tauri/Cargo.toml

