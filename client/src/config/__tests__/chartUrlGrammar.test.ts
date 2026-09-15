import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

import { describe, expect, it } from "vitest";

import { isOpenableExternalUrl } from "../../services/openExternal";
import { parseWebSocketUrl } from "../multiplayerServer";

// The Helm chart refuses an address the client would silently discard. That only
// holds while the chart's grammar for a value admits no more than the client's
// validator for that value, and the two live in different languages, so the
// grammar is read out of the template here rather than copied: a change to one
// without the other fails this test instead of drifting quietly.

// Walk up from the working directory so this resolves whether the suite runs
// from client/ or from the repo root; throws rather than silently skipping.
const HELPERS = (() => {
  const rel = "deploy/helm/phase-server/templates/_helpers.tpl";
  for (let dir = process.cwd(); ; dir = dirname(dir)) {
    const candidate = resolve(dir, rel);
    if (existsSync(candidate)) return candidate;
    if (dirname(dir) === dir) throw new Error(`could not locate ${rel} from ${process.cwd()}`);
  }
})();

function capture(src: string, pattern: RegExp, what: string): string {
  const m = src.match(pattern);
  if (!m) throw new Error(`no ${what} found in ${HELPERS}`);
  return m[1];
}

// The chart's verdict on web.<key>: that validator's anchored shape around the
// shared authority grammar, minus hosts with a punycode label.
function chartAccepts(key: string): (value: string) => boolean {
  const src = readFileSync(HELPERS, "utf8");
  const authority = capture(
    src,
    /define "phase-server\.urlAuthorityPattern" -\}\}\n\{\{- `([^`]+)` -\}\}/,
    "urlAuthorityPattern raw string",
  );
  const punycode = new RegExp(
    capture(
      src,
      /define "phase-server\.refusePunycodeHost" -\}\}\n\{\{- if regexMatch `([^`]+)` \.url -\}\}/,
      "refusePunycodeHost raw string",
    ),
  );
  const start = src.search(new RegExp(`\\$url := \\.Values\\.web\\.${key} -\\}\\}`));
  if (start < 0) throw new Error(`no web.${key} validator found in ${HELPERS}`);
  const end = src.indexOf('{{- define "', start);
  const body = src.slice(start, end < 0 ? undefined : end);
  // The verdict below subtracts the refusal, so a validator that stopped
  // performing it would otherwise still pass.
  if (!body.includes(`include "phase-server.refusePunycodeHost" (dict "key" "web.${key}"`)) {
    throw new Error(`the web.${key} validator does not include phase-server.refusePunycodeHost`);
  }
  const shape = new RegExp(
    capture(body, /\$re := printf `([^`]+)`/, `web.${key} shape`).replace("%s", () => authority),
  );
  return (v) => shape.test(v) && !punycode.test(v);
}

// Generated, not hand-listed. The first version of this guard listed sample
// addresses and missed a whole class — bracketed hosts with two elisions, and
// ones with too few groups and no elision — because nobody thought to write
// them down. Enumerating the shape instead of the examples is what makes the
// subset claim mean something.
const bracketed = new Set<string>();
for (let n = 1; n <= 10; n++) bracketed.add(Array(n).fill("1").join(":"));
for (let a = 0; a <= 4; a++)
  for (let b = 0; b <= 4; b++)
    bracketed.add(`${Array(a).fill("1").join(":")}::${Array(b).fill("2").join(":")}`);
for (let a = 0; a <= 3; a++)
  for (let b = 0; b <= 3; b++)
    for (let c = 0; c <= 3; c++)
      bracketed.add(
        `${Array(a).fill("1").join(":")}::${Array(b).fill("2").join(":")}::${Array(c).fill("3").join(":")}`,
      );
for (const h of [
  "::1", "::", "::ffff:192.168.1.1", "1:2:3:4:5:6:7:8", "2001:db8::8a2e:370:7334",
  "gggg::1", "1:2:3:4:5:6:7:8:9", "12345::1", "1::2::3", "::ffff:999.1.1.1", "x",
]) bracketed.add(h);

// Dotted-numeric hosts are the second generated class. URL parsing decides a
// host is an IPv4 attempt from its final label, so these are not hostnames
// that happen to contain digits — they are addresses that fail to parse.
const numeric = new Set<string>();
const MAGS = ["0", "1", "99", "127", "192", "255", "256", "999", "1000", "65535",
              "4294967295", "4294967296", "01", "0x7f", "0xff", "00"];
for (const n of [1, 2, 3, 4, 5])
  for (const m of MAGS) numeric.add(Array(n).fill(m).join("."));
for (const h of ["192.168.1.5", "255.255.255.255", "0.0.0.0", "1.2.3", "127.1",
                 "2130706433", "1.2.3.4.5", "999.999.999.999", "256.1.1.1",
                 "0x7f.0.0.1", "example.com", "sub.example.com", "localhost",
                 "host-1.example.com"]) numeric.add(h);

// URL parsing throws on an xn-- label that is not valid punycode, in any label
// and either case; the last host is valid punycode, which the chart may refuse.
const punycode = ["xn--a.example", "XN--a.example", "a.xn--a", "xn--.example", "xn--bcher-kva.example"];

const HOSTS = [...numeric, ...[...bracketed].map((h) => `[${h}]`), ...punycode];

const ROWS = [
  {
    key: "defaultMultiplayerServerUrl",
    clientAccepts: (v: string) => parseWebSocketUrl(v) !== null,
    corpus: [
      ...HOSTS.map((h) => `wss://${h}/ws`),
      "wss://play.example.com/ws",
      "ws://192.168.1.5:9374/ws",
      "wss://play.example.com/ws?region=eu",
      "wss://play.example.com:65535/ws",
      "wss://play.example.com:0/ws",
      "wss://play.example.com:abc/ws",
      "wss://play.example.com:99999/ws",
      "wss://play.example.com:-1/ws",
      "wss://[::1/ws",
      "wss://[]/ws",
      "wss://]::1[/ws",
      "wss://:9374/ws",
      "wss://@/ws",
      "wss://%00.com/ws",
      "wss://play.example.com bad",
      "wss://play.example.com\tbad",
      "wss://play.example.com/ws#lobby",
      "wss://play.example.com/ws#",
      "https://play.example.com",
      "play.example.com",
      "wss://",
    ],
    ordinary: ["wss://play.example.com/ws", "ws://192.168.1.5:9374/ws", "wss://[::1]:9374/ws"],
  },
  {
    key: "previewSiteUrl",
    clientAccepts: isOpenableExternalUrl,
    corpus: [
      // The shape admits a fragment, so every generated host is tried with one.
      ...HOSTS.flatMap((h) => [`https://${h}/`, `http://${h}/p?q=1#f`]),
      "https://preview.example.com:65535/",
      "https://preview.example.com:0/",
      "https://preview.example.com:abc/",
      "https://preview.example.com:99999/",
      "https://preview.example.com:-1/",
      "https://[::1/",
      "https://[]/",
      "https://]::1[/",
      "https://:8443/",
      "https://@/",
      "https://%00.com/",
      "https://preview.example.com/ bad",
      "https://preview.example.com/\tbad",
      "https://preview.example.com/#",
      "javascript:alert(1)",
      "wss://preview.example.com",
      "phase-preview.example.com",
      "https://",
    ],
    ordinary: [
      "https://phase-preview.example.com",
      "http://192.168.1.5:8080/",
      "https://[::1]:8443/p",
      "https://preview.example.com/play?x=1#top",
    ],
  },
];

describe.each(ROWS)("chart web.$key grammar vs the client", ({ key, clientAccepts, corpus, ordinary }) => {
  it("never admits an address the client would discard", () => {
    const admitted = corpus.filter(chartAccepts(key));
    expect(admitted.length).toBeGreaterThan(0);
    expect(admitted.filter((v) => !clientAccepts(v))).toEqual([]);
  });

  // Without this the case above passes for a chart regex that accepts nothing.
  it("still admits the ordinary addresses operators configure", () => {
    const accepts = chartAccepts(key);
    for (const v of ordinary) {
      expect(accepts(v), v).toBe(true);
      expect(clientAccepts(v), v).toBe(true);
    }
  });
});
