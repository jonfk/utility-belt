const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const { runInNewContext } = require("node:vm");

const source = readFileSync(join(__dirname, "background.js"), "utf8");

for (const [name, browser, namespaces] of [
  ["Firefox preserves the filename and container", "firefox", ["browser", "chrome"]],
  ["Chrome omits the filename", "chrome", ["chrome"]],
  ["Chrome with the browser namespace still omits the filename", "chrome", ["browser", "chrome"]],
]) {
  test(name, async () => {
    const manifest = JSON.parse(
      readFileSync(join(__dirname, `manifest.${browser}.json`), "utf8"),
    );
    let onClick;
    const downloads = [];
    const errors = [];
    const api = {
      runtime: {
        getManifest: () => manifest,
        onInstalled: { addListener() {} },
      },
      contextMenus: {
        onClicked: { addListener(listener) { onClick = listener; } },
      },
      scripting: {
        async executeScript() {
          return [{ result: { url: "https://example.com/video.mp4", filename: "Page title.mp4" } }];
        },
      },
      downloads: { async download(options) { downloads.push({ ...options }); } },
    };
    runInNewContext(source, {
      ...Object.fromEntries(namespaces.map((namespace) => [namespace, api])),
      console: { error: (...args) => errors.push(args) },
    });

    const tab = { id: 1 };
    const expected = { url: "https://example.com/video.mp4", saveAs: true };
    if (browser === "firefox") {
      tab.cookieStoreId = "firefox-container-1";
      expected.filename = "Page title.mp4";
      expected.cookieStoreId = tab.cookieStoreId;
    }
    await onClick({ menuItemId: "trigger-vid-dl" }, tab);

    assert.deepEqual(errors, []);
    assert.deepEqual(downloads, [expected]);
  });
}
