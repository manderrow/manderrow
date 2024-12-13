// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![deny(unused_must_use)]

fn main() -> anyhow::Result<()> {
    manderrow_lib::main()
}
