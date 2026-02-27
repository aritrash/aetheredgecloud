build:
	cargo build

img: build
	llvm-objcopy -O binary target/aarch64-unknown-none/debug/aether-edgecloud edgecloud.img

run: img
	qemu-system-aarch64 \
		-M virt,gic-version=3,highmem=off \
		-cpu max \
		-m 1G \
		-serial stdio \
		-display none \
		-machine virtualization=on \
		-kernel edgecloud.img

clean:
	cargo clean
	rm -f edgecloud.img