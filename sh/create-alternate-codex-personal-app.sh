#!/bin/sh
set -eu

APP_NAME="${APP_NAME:-Codex Personal}"
BUNDLE_ID="${BUNDLE_ID:-local.codex.personal}"
APP_DIR="${APP_DIR:-$HOME/Applications/${APP_NAME}.app}"
CODEX_APP="${CODEX_APP:-/Applications/Codex.app}"
CODEX_BIN="${CODEX_BIN:-$CODEX_APP/Contents/MacOS/Codex}"
ALT_CODEX_HOME="${ALT_CODEX_HOME:-$HOME/.local/share/codex-personal}"
LAUNCHER_NAME="${LAUNCHER_NAME:-codex-personal-launcher}"

if [ ! -x "$CODEX_BIN" ]; then
  echo "error: Codex binary not found or not executable: $CODEX_BIN" >&2
  exit 1
fi

mkdir -p "$ALT_CODEX_HOME"
mkdir -p "$APP_DIR/Contents/MacOS" "$APP_DIR/Contents/Resources"

cat > "$APP_DIR/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>${APP_NAME}</string>

  <key>CFBundleDisplayName</key>
  <string>${APP_NAME}</string>

  <key>CFBundleIdentifier</key>
  <string>${BUNDLE_ID}</string>

  <key>CFBundleExecutable</key>
  <string>${LAUNCHER_NAME}</string>

  <key>CFBundlePackageType</key>
  <string>APPL</string>

  <key>CFBundleVersion</key>
  <string>1</string>

  <key>CFBundleShortVersionString</key>
  <string>1.0</string>

  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
</dict>
</plist>
EOF

cat > "$APP_DIR/Contents/MacOS/$LAUNCHER_NAME" <<EOF
#!/bin/sh
CODEX_HOME="$ALT_CODEX_HOME"
export CODEX_HOME
exec "$CODEX_BIN" "\$@"
EOF

chmod +x "$APP_DIR/Contents/MacOS/$LAUNCHER_NAME"

# Reuse Codex's icon if one exists.
ICON_SRC=""
if [ -d "$CODEX_APP/Contents/Resources" ]; then
  ICON_SRC="$(find "$CODEX_APP/Contents/Resources" -maxdepth 1 -name '*.icns' 2>/dev/null | sed -n '1p')"
fi

if [ -n "$ICON_SRC" ]; then
  cp "$ICON_SRC" "$APP_DIR/Contents/Resources/CodexPersonal.icns"

  if command -v /usr/libexec/PlistBuddy >/dev/null 2>&1; then
    /usr/libexec/PlistBuddy -c "Add :CFBundleIconFile string CodexPersonal" "$APP_DIR/Contents/Info.plist" 2>/dev/null || \
    /usr/libexec/PlistBuddy -c "Set :CFBundleIconFile CodexPersonal" "$APP_DIR/Contents/Info.plist" 2>/dev/null || true
  fi
fi

if command -v plutil >/dev/null 2>&1; then
  plutil -lint "$APP_DIR/Contents/Info.plist" >/dev/null
fi

touch "$APP_DIR"

LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
if [ -x "$LSREGISTER" ]; then
  "$LSREGISTER" -f "$APP_DIR" 2>/dev/null || true
fi

echo "Created or updated: $APP_DIR"
echo "CODEX_HOME: $ALT_CODEX_HOME"
echo "Test with:"
echo "  open \"$APP_DIR\""
