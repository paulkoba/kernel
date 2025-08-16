set -e

cargo build -p kernel --target x86_64-failos.json --release -Z build-std=core,compiler_builtins,alloc
cargo run -p builder --release

qemu-system-x86_64 \
    -drive format=raw,file=boot.img \
    -cpu host \
    -serial stdio \
    -m 2G \
    -enable-kvm \
    -smp sockets=1,cores=1,threads=2