// Create the right-click menu on install
browser.runtime.onInstalled.addListener(() => {
  browser.contextMenus.create({
    id: "trigger-vid-dl",
    title: "Trigger Vid DL",
    contexts: ["page", "video"], // show on page or when right-clicking a <video>
  });
});

// When clicked, inject a tiny extractor in the page
browser.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== "trigger-vid-dl" || !tab?.id) return;

  try {
    const [{ result }] = await browser.scripting.executeScript({
      target: { tabId: tab.id },
      func: () => {
        // --- runs in the page ---
        const sanitize = (s) =>
          (s || "video")
            .replace(/https?:\/\/\S+|www\.\S+/gi, " ")
            .replace(/[\\\/:*?"<>|]+/g, " ")
            .replace(/\s+/g, " ")
            .trim()
            .slice(0, 180) || "video";

        // Find #player_el (it might be the <video> itself or a container)
        const root = document.querySelector("#player_el");
        let video = null;
        if (root) {
          video =
            root.tagName?.toLowerCase() === "video"
              ? root
              : root.querySelector("video");
        }
        if (!video) return { error: "No <video> under #player_el" };

        // Prefer currentSrc, fall back to src or first <source>
        let src = video.currentSrc || video.src;
        if (!src) {
          const source = video.querySelector("source[src]");
          if (source) src = source.getAttribute("src");
        }
        if (!src) return { error: "Video has no src" };

        // Resolve relative URL against the page URL
        const abs = new URL(src, location.href).toString();

        // Derive extension from the URL (fallback to mp4)
        const path = new URL(abs).pathname;
        const m = path.match(/\.([a-z0-9]+)(?=$|\?)/i);
        let ext = (m && m[1].toLowerCase()) || "mp4";
        if (ext.length > 6) ext = "mp4"; // guard against weird query-only endings

        const base = sanitize(document.title);
        const filename = `${base}.${ext}`;

        return { url: abs, filename };
      },
    });

    if (result?.error) {
      console.warn("[Trigger Vid DL]", result.error);
      return;
    }

    // Kick off the real browser download (uses session/Container cookies)
    await browser.downloads.download({
      url: result.url,
      filename: result.filename,
      saveAs: true,
      cookieStoreId: tab.cookieStoreId, // Firefox-only; keeps Container auth
    });
  } catch (err) {
    console.error("[Trigger Vid DL] failed:", err);
  }
});
