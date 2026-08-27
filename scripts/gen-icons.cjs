#!/usr/bin/env node
/**
 * gen-icons.cjs - Tauri app-icon generator (uses @resvg/resvg-js)
 *
 * Usage:
 *   node scripts/gen-icons.cjs          # Normal mode
 *   node scripts/gen-icons.cjs --silent # Quiet mode
 */

"use strict";

const { Resvg } = require("@resvg/resvg-js");
const fs = require("fs");
const path = require("path");
const { execSync } = require("child_process");

const ROOT = path.resolve(__dirname, "..");
const ICONS_DIR = path.join(ROOT, "src-tauri", "icons");
const SOURCE_PNG = path.join(ICONS_DIR, "icon.png");
const IS_SILENT = process.argv.includes("--silent");

const log = (msg) => !IS_SILENT && console.log(msg);
const error = (msg) => console.error(msg);

const ICON_SVG = `<svg width="240" height="240" viewBox="0 0 200 200" fill="none" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="shadow_bold_full" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="4" stdDeviation="6" flood-color="#831843" flood-opacity="0.3"/>
    </filter>
  </defs>
  <g filter="url(#shadow_bold_full)" stroke="#EC4899" stroke-width="12" stroke-linecap="round" stroke-linejoin="round">
    <path d="M100 185C100 185 185 130 185 80C185 45 155 25 125 25C108 25 100 38 100 38C100 38 92 25 75 25C45 25 15 45 15 80C15 130 100 185 100 185Z"/>
    <path d="M100 160C100 160 160 115 160 80C160 55 142 42 125 42C112 42 100 50 100 50C100 50 88 42 75 42C58 42 40 55 40 80C40 115 100 160 100 160Z" opacity="0.6"/>
    <path d="M100 135C100 135 135 105 135 80C135 68 128 60 120 60C110 60 100 68 100 68C100 68 90 60 80 60C72 60 65 68 65 80C65 105 100 135 100 135Z" opacity="0.3"/>
  </g>
</svg>`;

(async () => {
  try {
    log("Rendering SVG → icon.png (1024×1024)…");

    const resvg = new Resvg(ICON_SVG, {
      fitTo: { mode: "width", value: 1024 },
      imageRendering: 1,
      shapeRendering: 2,
      textRendering: 2,
    });

    const pngBuffer = resvg.render().asPng();

    fs.mkdirSync(ICONS_DIR, { recursive: true });
    fs.writeFileSync(SOURCE_PNG, pngBuffer);
    log(`Saved ${path.basename(SOURCE_PNG)} (${Math.round(pngBuffer.length / 1024)} KB)`);

    log("Running tauri icon generator…");
    execSync(`bun run tauri -- icon "${SOURCE_PNG}"`, {
      cwd: ROOT,
      stdio: IS_SILENT ? "pipe" : "inherit",
      timeout: 120_000,
    });

    log("All icons generated successfully in src-tauri/icons/");
    log("Rebuild the app to apply: bun run tauri -- build");
  } catch (err) {
    error("Process failed:");
    error(err.message || err);
    process.exit(1);
  }
})();
