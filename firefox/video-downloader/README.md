# Video Downloader Extension

A Firefox extension that enables easy downloading of videos from web pages while preserving authentication through Firefox Container cookies.

## Overview

This extension adds a context menu option to download videos directly from web pages. It's specifically designed to work with pages that use a `#player_el` element for video playback and maintains session authentication by preserving Firefox Container cookies during download.

## Features

- **Context Menu Integration**: Right-click anywhere on a page or on a video element to access the download option
- **Automatic Video Detection**: Intelligently locates video elements within `#player_el` containers
- **Smart Filename Generation**: Automatically creates clean filenames from page titles:
  - Removes URLs and special characters
  - Truncates to 180 characters maximum
  - Preserves video file extension from source URL
- **Multi-format Support**: Recognizes common video formats (mp4, webm, mov, avi, mkv, flv, wmv, m4v, mpeg, mpg, ogv, 3gp, ts, m3u8)
- **Firefox Container Support**: Maintains authentication by preserving `cookieStoreId` during downloads, crucial for downloading videos from authenticated sessions

## Usage

1. Navigate to a page with a video you want to download
2. Right-click on the page or video element
3. Select "Trigger Vid DL" from the context menu
4. Choose where to save the file in the browser's download dialog

The extension will automatically:
- Find the video element under `#player_el`
- Extract the video source URL (checking `currentSrc`, `src`, and `<source>` elements)
- Generate a sanitized filename based on the page title
- Initiate the download with proper authentication cookies

## Technical Details

### Architecture

- **Manifest Version**: 3
- **Background Script**: Handles context menu creation and orchestrates the download process
- **Content Script Injection**: Uses `browser.scripting.executeScript` to extract video information directly from the page DOM

### Permissions

- `contextMenus`: Create the right-click menu option
- `downloads`: Initiate file downloads
- `scripting`: Inject code into web pages to extract video information
- `activeTab`: Access the current tab for script injection
- `cookies`: Preserve authentication cookies during download (via `cookieStoreId`)

### Video Detection Logic

The extension follows this priority order:
1. Locates the `#player_el` element
2. Checks if it's a `<video>` element directly, or searches for one within
3. Extracts video URL from `currentSrc` → `src` → first `<source>` element
4. Resolves relative URLs against the page location
5. Detects file extension from URL pathname or defaults to `.mp4`

## Building

To build a packaged extension file (.xpi), use the included Makefile:

```bash
# Build the extension
make build

# Clean build artifacts
make clean

# Show help
make help
```

The build process uses Mozilla's `web-ext` tool via npx and creates a `.xpi` file in the `web-ext-artifacts/` directory. This file can be installed in Firefox.

**Note**: No additional dependencies need to be installed - `npx` will automatically download and run `web-ext` when needed.

## Installation

### Development/Unpacked Extension (Temporary)

1. Open Firefox and navigate to `about:debugging`
2. Click "This Firefox" in the left sidebar
3. Click "Load Temporary Add-on"
4. Navigate to this directory and select `manifest.json`

The extension will be loaded temporarily and will remain active until Firefox is restarted.

### Permanent Installation (Self-Hosted)

#### Option 1: Firefox Developer Edition or Nightly (Unsigned)

1. Build the extension: `make build`
2. Open Firefox Developer Edition or Nightly
3. Navigate to `about:config`
4. Set `xpinstall.signatures.required` to `false`
5. Navigate to `about:addons`
6. Click the gear icon and select "Install Add-on From File"
7. Select the `.xpi` file from `web-ext-artifacts/`

**Note**: Regular Firefox releases require signed extensions and won't allow this option.

#### Option 2: Load Unpacked (No Build Required)

For permanent installation without packaging:

1. Navigate to `about:config` in Firefox
2. Search for `extensions.autoDisableScopes` and set it to `0`
3. Navigate to `about:addons`
4. Click the gear icon and select "Install Add-on From File"
5. Select the `manifest.json` file directly

#### Option 3: Mozilla Add-ons Store (Signed)

For distribution to other users or permanent installation on regular Firefox:

1. Build the extension: `make build`
2. Create an account on [addons.mozilla.org](https://addons.mozilla.org)
3. Submit the `.xpi` file from `web-ext-artifacts/` for review
4. Once approved and signed, it can be installed on any Firefox browser

Mozilla's automated signing is also available for self-distribution without store listing.

## Limitations

- Currently hardcoded to look for videos under `#player_el` elements
- Designed primarily for Firefox (uses Firefox-specific `cookieStoreId` feature)
- Requires the video source to be accessible as a direct URL (may not work with DRM-protected or blob URLs in some cases)
