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
