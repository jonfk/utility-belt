.PHONY: build clean help

# Default target
help:
	@echo "Video Downloader Extension - Build Commands"
	@echo ""
	@echo "Available targets:"
	@echo "  make build   - Build the extension package (.xpi file)"
	@echo "  make clean   - Remove build artifacts"
	@echo "  make help    - Show this help message"

# Build the extension using web-ext
build:
	@echo "Building extension package..."
	npx web-ext build --overwrite-dest
	@echo ""
	@echo "Build complete! Package created in web-ext-artifacts/"

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	rm -rf web-ext-artifacts/
	@echo "Clean complete!"
