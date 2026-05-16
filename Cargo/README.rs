// Compiler flags for deployment:

RUSTFLAGS="-C target-cpu=native -C llvm-args=-force-vector-width=16 -C link-arg=-zrelro -C link-arg=-znow"
cargo build --release
