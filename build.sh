set -e

# Compile the init program assembly to ELF
echo "Compiling init program..."
as --64 -o kernel/programs/init.o kernel/programs/init.S
ld -m elf_x86_64 -Ttext 0x400000 --oformat binary -o kernel/programs/init.bin kernel/programs/init.o

cargo build -p kernel --release --target x86_64-failos.json -Z build-std=core,compiler_builtins,alloc
cargo run --release -p builder

qemu-system-x86_64 \
    -drive format=raw,file=boot.img \
    -cpu host \
    -serial stdio \
    -m 2G \
    -enable-kvm \
    -smp sockets=1,cores=1,threads=2