// Generates the icons the UI draws from Lucide's outlined SVG font: a font of them,
// assets/fonts/ManderrowIcons.ttf, and src/qml/core/Icons.qml, which maps their names to
// its characters. They are a font so that Qt Quick draws them as it does text, as sharply.
//
// To add an icon, add its Lucide name to NAMES and run `just icons`.

import * as path from "@std/path";
import svg2ttf from "svg2ttf";

const NAMES = [
  "arrow-down-narrow-wide",
  "arrow-up-wide-narrow",
  "check",
  "chevron-down",
  "chevron-left",
  "chevrons-right",
  "circle-alert",
  "circle-arrow-up",
  "circle-play",
  "clipboard",
  "copy",
  "download",
  "ellipsis-vertical",
  "external-link",
  "file-input",
  "folder-open",
  "forward",
  "globe",
  "grid-2x2",
  "hard-drive",
  "heart",
  "history",
  "info",
  "lightbulb",
  "list",
  "list-checks",
  "message-square-warning",
  "move-down",
  "move-right",
  "octagon-alert",
  "pin",
  "pin-off",
  "plus",
  "refresh-cw",
  "settings",
  "skull",
  "square-pen",
  "star",
  "trash",
  "trash-2",
  "triangle-alert",
  "x",
];

type Point = [number, number];
type Segment = { cmd: string; pts: Point[] };
type Contour = Segment[];

const FAMILY = "Manderrow Icons";

const scriptDir = path.dirname(path.fromFileUrl(import.meta.url));
const packageDir = path.join(scriptDir, "node_modules", "lucide-static");
const rootDir = path.join(scriptDir, "..", "..");
const qmlPath = path.join(rootDir, "src", "qml", "core", "Icons.qml");
const fontPath = path.join(rootDir, "assets", "fonts", "ManderrowIcons.ttf");

const font = await Deno.readTextFile(
  path.join(packageDir, "font", "lucide.svg"),
);
const license = await Deno.readTextFile(path.join(packageDir, "LICENSE"));
const em = Number(font.match(/units-per-em="([\d.]+)"/)![1]);
const ascent = Number(font.match(/ascent="([\d.]+)"/)![1]);

const glyphs = new Map<string, string>();
for (const [tag] of font.matchAll(/<glyph\b[^>]*>/g)) {
  const name = tag.match(/glyph-name="([^"]+)"/);
  const d = tag.match(/\sd="([^"]*)"/);
  if (name && d) glyphs.set(name[1], d[1]);
}

/// The glyph's contours, in the font's units.
function contours(d: string): Contour[] {
  const out: Contour[] = [];
  let current: Contour | undefined;
  for (const [, cmd, args] of d.matchAll(/([MLCZz])([^MLCZz]*)/g)) {
    if (cmd === "Z" || cmd === "z") continue;
    const nums = [...args.matchAll(/-?[\d.]+(?:e-?\d+)?/g)].map((m) =>
      Number(m[0])
    );
    if (nums.length % 2 !== 0) {
      throw new Error(`odd coordinates in ${cmd}${args}`);
    }
    const pts: Point[] = [];
    for (let i = 0; i < nums.length; i += 2) pts.push([nums[i], nums[i + 1]]);
    if (cmd === "M") {
      current = [{ cmd: "M", pts: pts.slice(0, 1) }];
      out.push(current);
      for (const pt of pts.slice(1)) current.push({ cmd: "L", pts: [pt] });
    } else if (cmd === "L") {
      for (const pt of pts) current!.push({ cmd: "L", pts: [pt] });
    } else {
      for (let i = 0; i < pts.length; i += 3) {
        current!.push({ cmd: "C", pts: pts.slice(i, i + 3) });
      }
    }
  }
  return out;
}

/// The contour traced the other way around, which flips whether it fills or cuts out.
function reversed(c: Contour): Contour {
  const ends = c.map(({ pts }) => pts[pts.length - 1]);
  const out: Contour = [{ cmd: "M", pts: [ends[ends.length - 1]] }];
  for (let i = c.length - 1; i > 0; i--) {
    const { cmd, pts } = c[i];
    out.push(
      cmd === "C"
        ? { cmd: "C", pts: [pts[1], pts[0], ends[i - 1]] }
        : { cmd: "L", pts: [ends[i - 1]] },
    );
  }
  return out;
}

function fmt(v: number): string {
  const s = v.toFixed(2).replace(/\.?0+$/, "");
  return s === "-0" ? "0" : s;
}

function toPath(cs: Contour[]): string {
  return cs
    .map((c) =>
      c.map(({ cmd, pts }) =>
        cmd + pts.map(([x, y]) => `${fmt(x)} ${fmt(y)}`).join(" ")
      ).join("") + "Z"
    )
    .join("");
}

/// The on-curve points with the control points, which is close enough for nesting tests.
function polygon(c: Contour): Point[] {
  return c.flatMap(({ pts }) => pts);
}

/// The polygon's area, positive or negative by the direction it goes around in.
function signedArea(poly: Point[]): number {
  let sum = 0;
  poly.forEach(([x0, y0], i) => {
    const [x1, y1] = poly[(i + 1) % poly.length];
    sum += x0 * y1 - x1 * y0;
  });
  return sum / 2;
}

function inside([x, y]: Point, poly: Point[]): boolean {
  let hit = false;
  poly.forEach(([x0, y0], i) => {
    const [x1, y1] = poly[(i + 1) % poly.length];
    if (y0 > y !== y1 > y && x < x0 + ((y - y0) * (x1 - x0)) / (y1 - y0)) {
      hit = !hit;
    }
  });
  return hit;
}

/// The contours of the filled variant: a shape with one hole has the hole filled in, and the
/// shapes inside that hole are knocked out of it solid, as solid icon sets draw them. Shapes
/// with several holes, like a globe's, keep them. The contours are turned so that they alternate
/// between filling and cutting out with their depth, as fonts' nonzero filling needs.
function filled(cs: Contour[]): Contour[] {
  const polys = cs.map(polygon);
  const signed = polys.map(signedArea);
  const areas = signed.map(Math.abs);
  const parent = polys.map((p, i) => {
    let best: number | null = null;
    polys.forEach((q, j) => {
      if (
        j !== i && areas[j] > areas[i] && inside(p[0], q) &&
        (best === null || areas[j] < areas[best])
      ) best = j;
    });
    return best;
  });
  const children = (i: number | null) =>
    cs.flatMap((_, j) => (parent[j] === i ? [j] : []));
  const outer = children(null);
  if (outer.length === 0) return cs;
  const fillSign = Math.sign(signed[outer[0]]);
  const kept: [number, Contour][] = [];
  const keep = (i: number, nested: number[], depth: number) => {
    const sign = depth % 2 === 0 ? fillSign : -fillSign;
    kept.push([i, Math.sign(signed[i]) === sign ? cs[i] : reversed(cs[i])]);
    if (nested.length === 1) nested = children(nested[0]);
    for (const j of nested) keep(j, children(j), depth + 1);
  };
  for (const i of outer) keep(i, children(i), 0);
  return kept.sort(([a], [b]) => a - b).map(([, c]) => c);
}

const missing = NAMES.filter((n) => !glyphs.has(n));
if (missing.length > 0) {
  throw new Error(`no such Lucide icons: ${missing.join(", ")}`);
}

// the icons take characters from the start of the Private Use Area, then their filled variants
let codepoint = 0xe000;
const fontGlyphs: string[] = [];
const icons: string[] = [];
const filledIcons: string[] = [];
const addGlyph = (
  name: string,
  glyphName: string,
  d: string,
  into: string[],
) => {
  const hex = (codepoint++).toString(16);
  fontGlyphs.push(
    `<glyph glyph-name="${glyphName}" unicode="&#x${hex};" horiz-adv-x="${em}" d="${d}"/>`,
  );
  into.push(`        "${name}": "\\u${hex}"`);
};
const solids: [string, Contour[]][] = [];
for (const name of [...NAMES].sort()) {
  const d = glyphs.get(name)!;
  addGlyph(name, name, d, icons);
  const cs = contours(d);
  const solid = filled(cs);
  if (solid.length !== cs.length) solids.push([name, solid]);
}
for (const [name, solid] of solids) {
  addGlyph(name, `${name}-filled`, toPath(solid), filledIcons);
}

const ttf = svg2ttf(
  `<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="${em}">
<font-face font-family="${FAMILY}" units-per-em="${em}" ascent="${ascent}" descent="${
    em - ascent
  }"/>
<missing-glyph horiz-adv-x="0"/>
${fontGlyphs.join("\n")}
</font></defs></svg>`,
  {
    copyright: "Lucide Icons and Contributors (ISC), and Cole Bemis (MIT)",
    // a fixed time, so that the font only changes when its icons do
    ts: 0,
    version: "1.0",
  },
);
await Deno.writeFile(fontPath, ttf.buffer);

const licenseComment = license
  .trim()
  .split("\n")
  .map((line) => ("// " + line).trimEnd())
  .join("\n");

await Deno.writeTextFile(
  qmlPath,
  `pragma Singleton
import QtQuick

// Generated by scripts/icons (\`just icons\`) from the outlined Lucide icons in lucide-static's SVG
// font, with filled variants derived from their outlines. The icons are the characters of the
// ${FAMILY} font (assets/fonts/ManderrowIcons.ttf).
//
${licenseComment}
QtObject {
    readonly property string family: "${FAMILY}"
    readonly property var icons: ({
${icons.join(",\n")}
    })
    /// The icons' filled variants, where they differ.
    readonly property var filled: ({
${filledIcons.join(",\n")}
    })
}
`,
);
console.error(
  `wrote ${icons.length} icons (${filledIcons.length} with filled variants) to ${fontPath} and ${qmlPath}`,
);
