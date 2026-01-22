# Lumbus - Build Configuration
.PHONY: all build release debug bundle clean install test icons dmg

# Default target
all: release

# Configuration
APP_NAME = Lumbus
BUNDLE_NAME = Lumbus.app
BINARY_NAME = lumbus

# Paths
BUILD_DIR = target/release
DEBUG_DIR = target/debug
SCRIPTS_DIR = scripts

# === Build Targets ===

# Development build
debug:
	cargo build

# Release build (optimized)
release:
	cargo build --release

# Run tests
test:
	cargo test

# === Bundle Targets ===

# Create .app bundle (release)
bundle: release
	$(SCRIPTS_DIR)/build-app.sh

# Create .app bundle (debug)
bundle-debug: debug
	$(SCRIPTS_DIR)/build-app.sh --debug

# === Icon Generation ===

# Generate .icns from source PNG (place 1024x1024 PNG at resources/icons/source.png)
icons:
	$(SCRIPTS_DIR)/generate-icons.sh

# === Installation ===

# Install to /Applications (may require sudo)
install: bundle
	@echo "Installing $(APP_NAME) to /Applications..."
	cp -R "$(BUILD_DIR)/$(BUNDLE_NAME)" /Applications/
	@echo "Installed successfully!"

# Install to user Applications folder
install-user: bundle
	@echo "Installing $(APP_NAME) to ~/Applications..."
	@mkdir -p ~/Applications
	cp -R "$(BUILD_DIR)/$(BUNDLE_NAME)" ~/Applications/
	@echo "Installed successfully!"

# === Code Signing ===

# Sign with Developer ID (set SIGN_IDENTITY env var or edit here)
sign: bundle
ifndef SIGN_IDENTITY
	$(error SIGN_IDENTITY is not set. Use: make sign SIGN_IDENTITY="Developer ID Application: Name (TEAMID)")
endif
	$(SCRIPTS_DIR)/build-app.sh --sign "$(SIGN_IDENTITY)"

# === Distribution ===

# Create DMG for distribution
dmg: bundle
	$(SCRIPTS_DIR)/build-dmg.sh

# === Cleanup ===

clean:
	cargo clean

# Remove only app bundles
clean-bundle:
	rm -rf "$(BUILD_DIR)/$(BUNDLE_NAME)"
	rm -rf "$(DEBUG_DIR)/$(BUNDLE_NAME)"

# === Development Helpers ===

# Run app directly (without bundle)
run:
	cargo run --profile dev

# Run release build
run-release: release
	./$(BUILD_DIR)/$(BINARY_NAME)

# Open app bundle
open: bundle
	open "$(BUILD_DIR)/$(BUNDLE_NAME)"

# Show app info
info: bundle
	@echo "=== App Bundle Info ==="
	@ls -la "$(BUILD_DIR)/$(BUNDLE_NAME)/Contents/"
	@echo ""
	@echo "=== Info.plist ==="
	@plutil -p "$(BUILD_DIR)/$(BUNDLE_NAME)/Contents/Info.plist"
