// Generates the Audis source app icon as a 1024x1024 PNG with no external
// dependencies, only Node's built-in zlib. `tauri icon` expands this single
// source into every size the installer and tray need.
//
// The mark mirrors src/components/Wordmark.tsx: an accent-blue rounded square
// (an approximated squircle) with three concentric white sound arcs opening to
// the right.

import { deflateSync } from "node:zlib";
import { writeFileSync } from "node:fs";

const SIZE = 1024;
const ACCENT = [0, 122, 255]; // systemBlue, light
const WHITE = [255, 255, 255];

const px = new Uint8Array(SIZE * SIZE * 4); // RGBA

function set(x, y, [r, g, b], a = 255) {
  if (x < 0 || y < 0 || x >= SIZE || y >= SIZE) return;
  const i = (y * SIZE + x) * 4;
  // Simple source-over alpha composite onto whatever is already there.
  const bg = px[i + 3] / 255;
  const fg = a / 255;
  const out = fg + bg * (1 - fg);
  if (out === 0) return;
  for (let c = 0; c < 3; c++) {
    const src = [r, g, b][c];
    px[i + c] = Math.round((src * fg + px[i + c] * bg * (1 - fg)) / out);
  }
  px[i + 3] = Math.round(out * 255);
}

// Squircle-ish rounded square via superellipse coverage, supersampled for a
// smooth edge.
const radius = SIZE * 0.225;
const cx = SIZE / 2;
const cy = SIZE / 2;
const half = SIZE / 2;
function roundedCoverage(x, y) {
  let hits = 0;
  const samples = 3;
  for (let sx = 0; sx < samples; sx++) {
    for (let sy = 0; sy < samples; sy++) {
      const px2 = x + (sx + 0.5) / samples - 0.5;
      const py2 = y + (sy + 0.5) / samples - 0.5;
      const dx = Math.abs(px2 - cx);
      const dy = Math.abs(py2 - cy);
      const inset = half - radius;
      if (dx <= inset || dy <= inset) {
        hits++;
      } else {
        const ox = dx - inset;
        const oy = dy - inset;
        if (ox * ox + oy * oy <= radius * radius) hits++;
      }
    }
  }
  return hits / (samples * samples);
}

for (let y = 0; y < SIZE; y++) {
  for (let x = 0; x < SIZE; x++) {
    const cov = roundedCoverage(x, y);
    if (cov > 0) set(x, y, ACCENT, Math.round(cov * 255));
  }
}

// Sound source dot and three arcs, drawn as stroked circles clipped to the
// right half so they read as wavefronts leaving a point.
const originX = SIZE * 0.34;
const originY = SIZE * 0.5;
const stroke = SIZE * 0.052;

function disc(cxx, cyy, r) {
  for (let y = Math.floor(cyy - r); y <= cyy + r; y++) {
    for (let x = Math.floor(cxx - r); x <= cxx + r; x++) {
      const d = Math.hypot(x - cxx, y - cyy);
      if (d <= r) set(x, y, WHITE);
    }
  }
}
disc(originX, originY, SIZE * 0.045);

// Each arc is a sequence of overlapping alpha discs stepped along a -60°..60°
// sweep opening to the right. Nearer arcs are opaque; far arcs fade, giving the
// sound-propagation feel without a real vector renderer.
function arcAlpha(r, alpha) {
  const steps = Math.ceil(r * 6);
  for (let s = 0; s <= steps; s++) {
    const t = (-Math.PI / 3) + (s / steps) * ((2 * Math.PI) / 3);
    const ax = originX + Math.cos(t) * r;
    const ay = originY + Math.sin(t) * r;
    const rr = stroke / 2;
    for (let y = Math.floor(ay - rr); y <= ay + rr; y++) {
      for (let x = Math.floor(ax - rr); x <= ax + rr; x++) {
        if (Math.hypot(x - ax, y - ay) <= rr) set(x, y, WHITE, alpha);
      }
    }
  }
}
arcAlpha(SIZE * 0.34, 102);
arcAlpha(SIZE * 0.25, 178);
arcAlpha(SIZE * 0.16, 255);

// ---- PNG encode ----
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, "ascii");
  const body = Buffer.concat([typeBuf, data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body) >>> 0, 0);
  return Buffer.concat([len, body, crc]);
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c >>> 0;
  }
  return table;
})();
function crc32(buf) {
  let c = 0xffffffff;
  for (let i = 0; i < buf.length; i++) c = CRC_TABLE[(c ^ buf[i]) & 0xff] ^ (c >>> 8);
  return c ^ 0xffffffff;
}

const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(SIZE, 0);
ihdr.writeUInt32BE(SIZE, 4);
ihdr[8] = 8; // bit depth
ihdr[9] = 6; // colour type RGBA
// filter/compression/interlace default 0

// Add a per-row filter byte (0 = none).
const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
for (let y = 0; y < SIZE; y++) {
  raw[y * (SIZE * 4 + 1)] = 0;
  px.subarray(y * SIZE * 4, (y + 1) * SIZE * 4).forEach((v, i) => {
    raw[y * (SIZE * 4 + 1) + 1 + i] = v;
  });
}
const idat = deflateSync(raw, { level: 9 });

const png = Buffer.concat([
  Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  chunk("IHDR", ihdr),
  chunk("IDAT", idat),
  chunk("IEND", Buffer.alloc(0)),
]);

const out = process.argv[2] ?? "app-icon.png";
writeFileSync(out, png);
console.log(`wrote ${out} (${SIZE}x${SIZE}, ${png.length} bytes)`);
