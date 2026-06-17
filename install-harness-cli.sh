cargo clean
cargo build --release -p harness-cli
cp target/release/harness-cli _harness/bin/harness-cli
