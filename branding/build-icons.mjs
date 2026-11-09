// Renders branding/logo.svg (and logo-small.svg for 32px and below, if present) into the app,
// installer and landing page icons. Needs Node 22+ and a local Chrome. No npm packages.
//   node branding/build-icons.mjs
import { spawn } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const CHROME = process.env.CHROME ?? "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
const PORT = 9344;
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];
const PNG_SIZE = 512;
const FAVICON_SIZES = [16, 32, 48];

const large = join(root, "branding", "logo.svg");
const small = existsSync(join(root, "branding", "logo-small.svg"))
  ? join(root, "branding", "logo-small.svg")
  : large;
const svgFor = (size) => (size <= 32 ? small : large);

const profile = join(tmpdir(), "cliptrace-icons-profile");
rmSync(profile, { recursive: true, force: true });
const chrome = spawn(
  CHROME,
  [
    "--headless=new", "--disable-gpu", "--hide-scrollbars", "--no-first-run",
    "--allow-file-access-from-files",
    `--remote-debugging-port=${PORT}`, `--user-data-dir=${profile}`, "about:blank",
  ],
  { stdio: "ignore", windowsHide: true },
);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function targets() {
  for (let i = 0; i < 60; i++) {
    try {
      const r = await fetch(`http://127.0.0.1:${PORT}/json`);
      if (r.ok) return await r.json();
    } catch { /* Chrome is still starting */ }
    await sleep(250);
  }
  throw new Error("Chrome did not start");
}

try {
  const page = (await targets()).find((t) => t.type === "page");
  const ws = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  let id = 0;
  const pending = new Map();
  ws.onmessage = (m) => {
    const d = JSON.parse(m.data);
    if (d.id && pending.has(d.id)) { pending.get(d.id)(d); pending.delete(d.id); }
  };
  const send = (method, params = {}) => new Promise((res, rej) => {
    const i = ++id;
    pending.set(i, (d) => (d.error ? rej(new Error(`${method}: ${d.error.message}`)) : res(d.result)));
    ws.send(JSON.stringify({ id: i, method, params }));
  });

  await send("Page.enable");
  await send("Emulation.setDefaultBackgroundColorOverride", { color: { r: 0, g: 0, b: 0, a: 0 } });

  /** Render an SVG at exactly size x size, with a transparent background. */
  async function render(svgPath, size) {
    const html = `<!doctype html><body style="margin:0;background:transparent"><img id="i" src="${pathToFileURL(svgPath).href}" width="${size}" height="${size}" style="display:block"></body>`;
    const htmlPath = join(profile, `r${size}.html`);
    mkdirSync(profile, { recursive: true });
    writeFileSync(htmlPath, html);
    await send("Emulation.setDeviceMetricsOverride", { width: size, height: size, deviceScaleFactor: 1, mobile: false });
    await send("Page.navigate", { url: pathToFileURL(htmlPath).href });
    for (let i = 0; i < 40; i++) {
      const r = await send("Runtime.evaluate", { expression: "document.getElementById('i') && document.getElementById('i').complete && document.getElementById('i').naturalWidth > 0", returnByValue: true });
      if (r.result.value) break;
      await sleep(100);
    }
    const shot = await send("Page.captureScreenshot", { format: "png", clip: { x: 0, y: 0, width: size, height: size, scale: 1 }, captureBeyondViewport: false });
    return Buffer.from(shot.data, "base64");
  }

  /** Pack PNG images into one .ico (PNG-compressed entries, supported since Windows Vista). */
  function packIco(images) {
    const head = Buffer.alloc(6);
    head.writeUInt16LE(1, 2);
    head.writeUInt16LE(images.length, 4);
    const dir = Buffer.alloc(16 * images.length);
    let offset = 6 + dir.length;
    images.forEach(({ size, png }, i) => {
      const e = i * 16;
      dir[e] = size >= 256 ? 0 : size;
      dir[e + 1] = size >= 256 ? 0 : size;
      dir.writeUInt16LE(1, e + 4);
      dir.writeUInt16LE(32, e + 6);
      dir.writeUInt32LE(png.length, e + 8);
      dir.writeUInt32LE(offset, e + 12);
      offset += png.length;
    });
    return Buffer.concat([head, dir, ...images.map((i) => i.png)]);
  }

  const out = (...p) => join(root, ...p);
  const rendered = new Map();
  const get = async (size) => {
    if (!rendered.has(size)) rendered.set(size, await render(svgFor(size), size));
    return rendered.get(size);
  };

  const icoImages = [];
  for (const size of ICO_SIZES) icoImages.push({ size, png: await get(size) });
  writeFileSync(out("src-tauri", "icons", "icon.ico"), packIco(icoImages));
  writeFileSync(out("src-tauri", "icons", "icon.png"), await get(PNG_SIZE));

  const favImages = [];
  for (const size of FAVICON_SIZES) favImages.push({ size, png: await get(size) });
  writeFileSync(out("landing", "favicon.ico"), packIco(favImages));
  writeFileSync(out("landing", "favicon.svg"), readFileSync(large));

  console.log("wrote icon.ico, icon.png, favicon.ico, favicon.svg");
  ws.close();
} finally {
  chrome.kill();
}
