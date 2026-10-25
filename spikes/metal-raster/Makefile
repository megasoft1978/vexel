# Standalone spike; not part of the Cargo workspace.
BUILD := build
all: $(BUILD)/mr $(BUILD)/raster.metallib
$(BUILD)/raster.air: raster.metal
	@mkdir -p $(BUILD)
	xcrun metal -c -fno-fast-math -O2 raster.metal -o $@
$(BUILD)/raster.metallib: $(BUILD)/raster.air
	xcrun metallib $< -o $@
$(BUILD)/mr: main.swift
	@mkdir -p $(BUILD)
	xcrun swiftc -Ounchecked main.swift -o $@
clean:
	rm -f $(BUILD)/mr $(BUILD)/raster.air $(BUILD)/raster.metallib
