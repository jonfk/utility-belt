# Video Downloader Extension

A Firefox and Chrome extension that downloads videos from web pages while preserving the browser session's authentication cookies. Firefox Container tabs are also supported.

## Features

- Adds **Trigger Vid DL** to the page and video context menus
- Finds a `<video>` within the page's `#player_el` element
- Uses `currentSrc`, `src`, or the first `<source>` element
- Generates a sanitized filename from the page title in Firefox
- Opens Chrome's Save As dialog in the last-used download directory
- Recognizes common video file extensions
- Uses the active browser profile's authentication cookies
- Preserves the Firefox Container cookie store when applicable

The source must be a directly downloadable URL. DRM-protected media and `blob:` URLs are not supported, and downloading an HLS playlist (`.m3u8`) does not combine its segments.

## Source layout

- `background.js`: shared extension logic
- `manifest.firefox.json`: Firefox Manifest V3 configuration
- `manifest.chrome.json`: Chrome Manifest V3 configuration
- `dist/firefox`: generated Firefox directory for temporary loading
- `dist/chrome`: generated Chrome directory for unpacked loading
- `dist/packages`: generated packaged artifacts

The browser manifests are kept separate because Firefox uses `background.scripts` and Gecko-specific settings, while Chrome uses an extension service worker. The shared JavaScript selects the available `browser` or `chrome` API namespace at runtime.

## Building

The build requires `zip` and Node.js with `npx`. Mozilla's `web-ext` is invoked through `npx`.

```bash
# Build both browsers
make build

# Or build one browser
make build-firefox
make build-chrome

# Only prepare directories for development loading
make prepare-firefox
make prepare-chrome

# Remove generated files
make clean
```

`make build` creates:

- `dist/packages/video-downloader-firefox.xpi`
- `dist/packages/video-downloader-chrome.zip`

## Development installation

### Firefox

1. Run `make prepare-firefox`.
2. Open `about:debugging`.
3. Select **This Firefox**, then **Load Temporary Add-on**.
4. Select `dist/firefox/manifest.json`.

The temporary extension remains installed until Firefox restarts. Permanent installation in regular Firefox requires a Mozilla-signed XPI. Developer Edition, Nightly, and ESR can load unsigned packages when signature enforcement is disabled.

### Chrome

1. Run `make prepare-chrome`.
2. Open `chrome://extensions`.
3. Enable **Developer mode**.
4. Select **Load unpacked** and choose `dist/chrome`.

The ZIP is intended for Chrome Web Store submission or distribution workflows; Chrome's development loader expects the unpacked directory rather than the ZIP.

## Usage

1. Navigate to a page containing a video under `#player_el`.
2. Right-click the page or video.
3. Select **Trigger Vid DL**.
4. Choose the destination in the Save As dialog.

Firefox pre-fills the sanitized page-title filename and remembers the last directory used by this extension. Chrome also remembers the last directory, but its extension API cannot combine that behavior with an extension-supplied filename. Chrome therefore derives the initial filename from the video's URL or response headers.

## Permissions

- `contextMenus`: add the right-click action
- `downloads`: start the download
- `scripting`: inspect the page DOM after the user invokes the action
- `activeTab`: grant access to the selected tab
- `cookies` (Firefox only): associate downloads with Firefox Container cookie stores
