set windows-powershell

export RUST_BACKTRACE := "1"
export RUST_LOG := "debug"

run *ARGS:
	cargo run {{ARGS}}

build *ARGS:
	cargo build --release {{ARGS}}

fmt:
	cargo fmt --all
	zig fmt agent

clean:
	cargo clean
	rm -vrf agent/.zig-cache
	rm -vrf agent/zig-out

test:
	cargo test --workspace

update:
	cargo update --verbose

check:
	cargo check --workspace

# Generates Strings.qml from the translations, and the QML module's types from the bridges in
# src/qt for qmlls and qmllint, as the build does, but without building the app. It builds in its
# own target directory, so that it doesn't wait for a build of the app, or rust-analyzer's.
gen:
	cargo run --target-dir target/tools -p manderrow-qml-gen

# Lints the QML. Takes qmllint options or files.
lint *ARGS: gen
	python3 -I scripts/lint-qml.py {{ARGS}}

run-script BIN *ARGS:
	cargo run --bin {{BIN}} -- {{ARGS}}

[working-directory: 'scripts/game-updater']
add-games:
	deno run add-games

[working-directory: 'scripts/game-updater']
scrape:
	deno run scrape


[working-directory: 'scripts/icons']
icons:
	deno run generate
