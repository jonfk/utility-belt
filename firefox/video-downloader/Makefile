.PHONY: build build-firefox build-chrome prepare-firefox prepare-chrome clean help

DIST_DIR := dist
FIREFOX_DIR := $(DIST_DIR)/firefox
CHROME_DIR := $(DIST_DIR)/chrome
PACKAGES_DIR := $(DIST_DIR)/packages

# Default target
help:
	@echo "Video Downloader Extension - Build Commands"
	@echo ""
	@echo "Available targets:"
	@echo "  make build           - Build Firefox and Chrome packages"
	@echo "  make build-firefox   - Build the Firefox .xpi package"
	@echo "  make build-chrome    - Build the Chrome .zip package"
	@echo "  make prepare-firefox - Create a Firefox directory for temporary loading"
	@echo "  make prepare-chrome  - Create a Chrome directory for Load unpacked"
	@echo "  make clean           - Remove generated build artifacts"
	@echo "  make help            - Show this help message"

build: build-firefox build-chrome

prepare-firefox:
	mkdir -p $(FIREFOX_DIR)
	cp background.js $(FIREFOX_DIR)/background.js
	cp manifest.firefox.json $(FIREFOX_DIR)/manifest.json

prepare-chrome:
	mkdir -p $(CHROME_DIR)
	cp background.js $(CHROME_DIR)/background.js
	cp manifest.chrome.json $(CHROME_DIR)/manifest.json

build-firefox: prepare-firefox
	mkdir -p $(PACKAGES_DIR)
	npx web-ext build --source-dir $(FIREFOX_DIR) --artifacts-dir $(PACKAGES_DIR) --filename video-downloader-firefox.xpi --overwrite-dest

build-chrome: prepare-chrome
	mkdir -p $(PACKAGES_DIR)
	rm -f $(PACKAGES_DIR)/video-downloader-chrome.zip
	cd $(CHROME_DIR) && zip -qr ../packages/video-downloader-chrome.zip .

clean:
	rm -rf $(DIST_DIR)
