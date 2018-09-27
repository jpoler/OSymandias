TARGET ?= aarch64-none-elf
CROSS ?= $(TARGET)

CC := $(CROSS)-gcc
TTYWRITE ?= ttywrite
PI_TTY ?= /dev/tty.SLAB_USBtoUART
CCFLAGS ?= -Wall -O2 -nostdlib -nostartfiles -ffreestanding -pie -fpie
LDFLAGS ?= --gc-sections -static -nostdlib -nostartfiles --no-dynamic-linker
XARGO ?= CARGO_INCREMENTAL=0 RUST_TARGET_PATH="$(shell pwd)" xargo
CARGO ?= cargo

LD_LAYOUT := ext/layout.ld

RUST_BINARY := $(shell cat Cargo.toml | grep name | cut -d\" -f 2 | tr - _)
RUST_BUILD_DIR := target/$(TARGET)
RUST_DEBUG_LIB := $(RUST_BUILD_DIR)/debug/lib$(RUST_BINARY).a
RUST_RELEASE_LIB := $(RUST_BUILD_DIR)/release/lib$(RUST_BINARY).a

RUST_LIB_DEPS = ../pi/src/* ../pi/src/*/** \
				../../1-shell/stack-vec/src/* \
				../../2-fs/fat32/src/* ../../2-fs/fat32/src/*/**

RUST_DEPS = Xargo.toml Cargo.toml build.rs $(LD_LAYOUT) src/* $(RUST_LIB_DEPS)
EXT_DEPS = $(BUILD_DIR)/init.o

FIRMWARE_DIR := ../files/firmware
BUILD_DIR := build
KERNEL := $(BUILD_DIR)/$(RUST_BINARY)
SD_CARD := /Volumes/boot
SD_KERNEL := $(SD_CARD)/kernel8.img
RUST_LIB := $(BUILD_DIR)/$(RUST_BINARY).a

.PHONY: all test clean check copy copy-umount watch install

VPATH = ext

all: $(KERNEL).hex $(KERNEL).bin

check:
	@$(XARGO) check --target=$(TARGET)

test:
	@$(CARGO) test

install: $(KERNEL).bin
	$(TTYWRITE) -m write -t 60 -i $< $(PI_TTY)

$(RUST_DEBUG_LIB): $(RUST_DEPS)
	@echo "+ Building $@ [xargo]"
	@$(XARGO) build --target=$(TARGET)

$(RUST_RELEASE_LIB): $(RUST_DEPS)
	@echo "+ Building $@ [xargo --release]"
	@$(XARGO) build --release --target=$(TARGET)

ifeq ($(DEBUG),1)
$(RUST_LIB): $(RUST_DEBUG_LIB) | $(BUILD_DIR)
	@cp $< $@
else
$(RUST_LIB): $(RUST_RELEASE_LIB) | $(BUILD_DIR)
	@cp $< $@
endif

$(BUILD_DIR):
	@mkdir -p $@

$(BUILD_DIR)/%.o: %.c | $(BUILD_DIR)
	@echo "+ Building $@ [cc $<]"
	@$(CC) $(CCFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: %.S | $(BUILD_DIR)
	@echo "+ Building $@ [as $<]"
	@$(CC) $(CCFLAGS) -c $< -o $@

$(KERNEL).elf: $(EXT_DEPS) $(RUST_LIB) | $(BUILD_DIR)
	@echo "+ Building $@ [ld $^]"
	@$(CROSS)-ld $(LDFLAGS) -T$(LD_LAYOUT) $^ -o $@

$(KERNEL).hex: $(KERNEL).elf | $(BUILD_DIR)
	@echo "+ Building $@ [objcopy $<]"
	@$(CROSS)-objcopy $< -O ihex $@

$(KERNEL).bin: $(KERNEL).elf | $(BUILD_DIR)
	@echo "+ Building $@ [objcopy $<]"
	@$(CROSS)-objcopy $< -O binary $@

clean:
	$(XARGO) clean
	rm -rf $(BUILD_DIR)

clean-sd:
	rm -rf $(SD_CARD)/*

copy-firmware:
	cp $(FIRMWARE_DIR)/* $(SD_CARD)

copy: all clean-sd copy-firmware
	rm -f $(SD_KERNEL)
	cp $(KERNEL).bin $(SD_KERNEL)

umount:
	diskutil umount $(SD_CARD)

copy-umount: copy umount
