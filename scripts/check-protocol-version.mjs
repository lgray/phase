import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const EXPECTED_PROTOCOL_VERSION = 59;
// The LOBBY message-set version. Deliberately separate from the full-game
// number above and deliberately NOT derived from it: a GameState-only bump must
// not move the lobby's compatibility window. See the assertions at the bottom.
const EXPECTED_LOBBY_PROTOCOL_VERSION = 4;
// The P2P wire version. A THIRD independent surface: host/guest first-contact
// frames carry it, and the same GameState shape change that moves
// EXPECTED_PROTOCOL_VERSION must move this one too. It was previously ungated
// here, so a full-game bump could ship with an unbumped P2P version and CI
// stayed green — a v(n-1) host and a v(n) guest would then complete a
// handshake and only fail when the incompatible payload arrived.
const EXPECTED_WIRE_PROTOCOL_VERSION = 43;

function extractVersion(source, pattern, label) {
  const match = source.match(pattern);
  if (!match) {
    throw new Error(`Could not find protocol version in ${label}`);
  }
  return Number(match[1]);
}

function requirePattern(source, pattern, label) {
  if (!pattern.test(source)) {
    throw new Error(`Required protocol form not found in ${label}`);
  }
}

function refusePattern(source, pattern, label) {
  if (pattern.test(source)) {
    throw new Error(`Superseded protocol version still named in ${label}`);
  }
}

const rustSource = readFileSync(
  resolve(root, "crates/lobby-broker/src/protocol.rs"),
  "utf8",
);
const serverCoreSource = readFileSync(
  resolve(root, "crates/server-core/src/protocol.rs"),
  "utf8",
);
const clientSource = readFileSync(
  resolve(root, "client/src/adapter/ws-adapter.ts"),
  "utf8",
);
const workerHelloGateSource = readFileSync(
  resolve(root, "lobby-worker/src/hello-gate.ts"),
  "utf8",
);
const p2pProtocolSource = readFileSync(
  resolve(root, "client/src/network/protocol.ts"),
  "utf8",
);
const p2pProtocolTestSource = readFileSync(
  resolve(root, "client/src/network/__tests__/protocol.test.ts"),
  "utf8",
);
const p2pAdapterTestSource = readFileSync(
  resolve(root, "client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts"),
  "utf8",
);

const rustVersion = extractVersion(
  rustSource,
  /pub\s+const\s+PROTOCOL_VERSION\s*:\s*u32\s*=\s*(\d+)\s*;/,
  "crates/lobby-broker/src/protocol.rs",
);
const clientVersion = extractVersion(
  clientSource,
  /export\s+const\s+PROTOCOL_VERSION\s*=\s*(\d+)\s*;/,
  "client/src/adapter/ws-adapter.ts",
);

requirePattern(
  rustSource,
  /pub\s+const\s+MIN_SUPPORTED_PROTOCOL\s*:\s*u32\s*=\s*PROTOCOL_VERSION\.saturating_sub\(1\)\s*;/,
  "crates/lobby-broker/src/protocol.rs",
);
requirePattern(
  serverCoreSource,
  /pub\s+const\s+MIN_SUPPORTED_PROTOCOL\s*:\s*u32\s*=\s*PROTOCOL_VERSION\s*;/,
  "crates/server-core/src/protocol.rs",
);
requirePattern(
  clientSource,
  /export\s+const\s+MIN_SUPPORTED_SERVER_PROTOCOL\s*=\s*PROTOCOL_VERSION\s*;/,
  "client/src/adapter/ws-adapter.ts",
);
requirePattern(
  workerHelloGateSource,
  /const\s+legacyMin\s*=\s*Math\.max\(0,\s*policy\.serverProtocolVersion\s*-\s*1\)\s*;/,
  "lobby-worker/src/hello-gate.ts",
);

// ── Authored vs derived: which constants may carry a bare integer ──────────
//
// The `requirePattern` calls above pin the FORM of one named formula each.
// This block closes the class those are members of: every protocol constant
// declared in the four files below is classified, and only the names listed
// here may carry an integer. A derived constant replaced by its correct
// current value passes every value comparison here and every relational
// assertion in the Rust and vitest suites, and only reds at the NEXT bump — on
// a change that has nothing to do with it. This catches it at the edit that
// introduces it, and an unlisted name with an integer right-hand side fails
// whether or not it existed when the list was written. Three ceilings: a
// right-hand side that is constant but not a decimal integer (hex, arithmetic,
// a block expression) reads as derived, and so does a decimal integer whose
// type suffix falls outside INTEGER_RHS's `[iu]<digits>` alphabet — `<n>usize`,
// `<n>isize` and TypeScript's `<n>n`; the name filter below lets a helper named
// neither PROTOCOL nor MIN_SUPPORTED hold the literal while a protocol
// constant derives from it; and the declaration regex sees one binding per
// `const` and ends the right-hand side at the first `;`, comment or not.
const AUTHORED_LITERALS = [
  [rustSource, "crates/lobby-broker/src/protocol.rs", [
    "LOBBY_PROTOCOL_VERSION",
    "MIN_SUPPORTED_LOBBY_PROTOCOL",
    "PROTOCOL_VERSION",
  ]],
  [serverCoreSource, "crates/server-core/src/protocol.rs", []],
  [clientSource, "client/src/adapter/ws-adapter.ts", [
    "LOBBY_PROTOCOL_VERSION",
    "MIN_SUPPORTED_SERVER_LOBBY_PROTOCOL",
    "PROTOCOL_VERSION",
  ]],
  [p2pProtocolSource, "client/src/network/protocol.ts", [
    "WIRE_PROTOCOL_VERSION",
  ]],
];

// Binding keyword, visibility, type annotation, digit separators and a cast
// are optional or free-form, so a literalization cannot hide behind `static`,
// `pub(crate)`, `: std::primitive::u32`, an underscore between digits,
// `<n> as u32`, or a comment.
const CONST_DECL =
  /(?:pub(?:\([^)]+\))?\s+|export\s+)?(?:const|static)\s+([A-Z][A-Z0-9_]*)\s*(?::\s*[^=;]+)?\s*=\s*([^;]+);/g;
const INTEGER_RHS = /^\d[\d_]*(_?[iu]\d+)?(\s+(as|satisfies)\s+[^=;]+)?$/;

for (const [source, label, authored] of AUTHORED_LITERALS) {
  const found = [...source.matchAll(CONST_DECL)]
    .filter(([, name]) => /PROTOCOL|MIN_SUPPORTED/.test(name))
    .filter(([, , rhs]) =>
      INTEGER_RHS.test(rhs.replace(/\/\*[\s\S]*?\*\/|\/\/[^\n]*/g, " ").trim()),
    )
    .map(([, name]) => name)
    .sort();
  const expected = [...authored].sort();
  if (found.join(" ") !== expected.join(" ")) {
    console.error(
      `Protocol constants with a bare-integer right-hand side in ${label} must be exactly [${expected.join(", ")}], found [${found.join(", ")}]. ` +
        `Every other protocol constant there derives from one of these and must stay an expression.`,
    );
    process.exit(1);
  }
}

if (rustVersion !== clientVersion) {
  console.error(
    `Protocol version mismatch: Rust=${rustVersion}, client=${clientVersion}`,
  );
  process.exit(1);
}

if (
  rustVersion !== EXPECTED_PROTOCOL_VERSION ||
  clientVersion !== EXPECTED_PROTOCOL_VERSION
) {
  console.error(
    `Protocol version must remain ${EXPECTED_PROTOCOL_VERSION}: Rust=${rustVersion}, client=${clientVersion}`,
  );
  process.exit(1);
}

// ── P2P wire protocol: the third surface ───────────────────────────────────
//
// Pinned here for the same reason the full-game number is: a `GameState` shape
// change crosses BOTH the WebSocket full-game wire and the P2P host/guest wire,
// and bumping only one leaves the other pairing to fail at payload-decode time
// instead of at the handshake. Gating both in one place makes "I bumped the
// protocol" mean all of it.

const wireProtocolVersion = extractVersion(
  p2pProtocolSource,
  /export\s+const\s+WIRE_PROTOCOL_VERSION\s*=\s*(\d+)\s*as\s+const\s*;/,
  "client/src/network/protocol.ts",
);

if (wireProtocolVersion !== EXPECTED_WIRE_PROTOCOL_VERSION) {
  console.error(
    `P2P wire protocol version must remain ${EXPECTED_WIRE_PROTOCOL_VERSION}: got ${wireProtocolVersion}. ` +
      `A GameState shape change must bump this alongside PROTOCOL_VERSION, not instead of it.`,
  );
  process.exit(1);
}

// ── Lobby protocol: a SEPARATE surface with its own version ────────────────
//
// `PROTOCOL_VERSION` versions the full-game GameState/GameAction wire surface.
// The lobby broker parses none of that, yet its accept-window used to be
// derived from that same number — so a GameState-only bump slid the lobby
// window and stranded every already-deployed client. These assertions keep the
// two surfaces genuinely independent.

const rustLobbyVersion = extractVersion(
  rustSource,
  /pub\s+const\s+LOBBY_PROTOCOL_VERSION\s*:\s*u32\s*=\s*(\d+)\s*;/,
  "crates/lobby-broker/src/protocol.rs",
);
const clientLobbyVersion = extractVersion(
  clientSource,
  /export\s+const\s+LOBBY_PROTOCOL_VERSION\s*=\s*(\d+)\s*;/,
  "client/src/adapter/ws-adapter.ts",
);
const rustLobbyFloor = extractVersion(
  rustSource,
  /pub\s+const\s+MIN_SUPPORTED_LOBBY_PROTOCOL\s*:\s*u32\s*=\s*(\d+)\s*;/,
  "crates/lobby-broker/src/protocol.rs",
);
const clientLobbyFloor = extractVersion(
  clientSource,
  /export\s+const\s+MIN_SUPPORTED_SERVER_LOBBY_PROTOCOL\s*=\s*(\d+)\s*;/,
  "client/src/adapter/ws-adapter.ts",
);

// The structural invariant, and the reason this block exists. Each of the four
// regexes above requires a bare integer literal on the right-hand side, and all
// four names sit on the authored-literal lists further up, so a future edit to
// `LOBBY_PROTOCOL_VERSION = PROTOCOL_VERSION - 1` (or any other expression)
// fails that classification first rather than silently re-coupling the two
// surfaces.

if (rustLobbyVersion !== clientLobbyVersion) {
  console.error(
    `Lobby protocol version mismatch: Rust=${rustLobbyVersion}, client=${clientLobbyVersion}`,
  );
  process.exit(1);
}

if (rustLobbyFloor !== clientLobbyFloor) {
  console.error(
    `Lobby protocol floor mismatch: Rust=${rustLobbyFloor}, client=${clientLobbyFloor}`,
  );
  process.exit(1);
}

if (rustLobbyVersion !== EXPECTED_LOBBY_PROTOCOL_VERSION) {
  console.error(
    `Lobby protocol version must remain ${EXPECTED_LOBBY_PROTOCOL_VERSION}: got ${rustLobbyVersion}. ` +
      `Bump it ONLY for a LobbyClientMessage/LobbyServerMessage shape change — never for a full-game bump.`,
  );
  process.exit(1);
}

if (rustLobbyFloor > rustLobbyVersion) {
  console.error(
    `Lobby floor ${rustLobbyFloor} exceeds the lobby version ${rustLobbyVersion}: no client could connect.`,
  );
  process.exit(1);
}

// ── Directory announcement shape: a FOURTH constant surface ────────────────
//
// `DIRECTORY_VERSION` versions the ANNOUNCEMENT shape — the `POST /announce`
// body Rust sends and the `GET /servers` envelope the client reads. It is
// unrelated to all three wire protocols above: none of the lobby, full-game or
// P2P message sets appears in a directory row. It moves only when
// `RawAnnouncement` / `DirectoryRow` change shape, and both sides must move
// together or a client silently ignores every listing.

const EXPECTED_DIRECTORY_VERSION = 1;

const directorySource = readFileSync(
  resolve(root, "crates/lobby-broker/src/directory.rs"),
  "utf8",
);
const clientDirectorySource = readFileSync(
  resolve(root, "client/src/services/serverDirectory.ts"),
  "utf8",
);

// Both regexes require a bare integer literal on the right-hand side, so a
// future `DIRECTORY_VERSION = SOMETHING + 1` trips "Could not find protocol
// version" rather than silently un-pinning the surface.
const rustDirectoryVersion = extractVersion(
  directorySource,
  /pub\s+const\s+DIRECTORY_VERSION\s*:\s*u32\s*=\s*(\d+)\s*;/,
  "crates/lobby-broker/src/directory.rs",
);
const clientDirectoryVersion = extractVersion(
  clientDirectorySource,
  /export\s+const\s+DIRECTORY_VERSION\s*=\s*(\d+)\s*;/,
  "client/src/services/serverDirectory.ts",
);

if (rustDirectoryVersion !== clientDirectoryVersion) {
  console.error(
    `Directory version mismatch: Rust=${rustDirectoryVersion}, client=${clientDirectoryVersion}`,
  );
  process.exit(1);
}

// Not redundant with the mismatch check above. A COORDINATED bump of both
// sides — exactly what an auto-merge of two branches can produce silently —
// passes consistency and fails here.
if (rustDirectoryVersion !== EXPECTED_DIRECTORY_VERSION) {
  console.error(
    `Directory version must remain ${EXPECTED_DIRECTORY_VERSION}: got ${rustDirectoryVersion}. ` +
      `Bump it ONLY for a RawAnnouncement/DirectoryRow shape change.`,
  );
  process.exit(1);
}
// ── Names that embed a version number ─────────────────────────────────────
//
// A name carrying a version goes stale silently: `assert_eq!(PROTOCOL_VERSION,
// <n>)` under `fn protocol_version_is_<n-1>` is green, and so is a handshake
// pair whose title still advertises the version before last. Each site
// requires the CURRENT number and refuses the SUPERSEDED one, both derived
// from the EXPECTED_* constants above, so a later bump edits the sources and
// those constants, never the patterns themselves. Ceiling: the refuse leg
// catches leftover text from the previous version, which is the defect a bump
// produces. Prose rewritten to some other wrong number is not a bump leftover
// and is not guarded here.
const P = EXPECTED_PROTOCOL_VERSION;
const W = EXPECTED_WIRE_PROTOCOL_VERSION;

requirePattern(serverCoreSource, new RegExp(`protocol_version_is_${P}(?![0-9])`),
  `crates/server-core/src/protocol.rs protocol_version_is_${P}`);
refusePattern(serverCoreSource, new RegExp(`protocol_version_is_${P - 1}(?![0-9])`),
  "crates/server-core/src/protocol.rs");

requirePattern(p2pProtocolTestSource, new RegExp(`\\bv${W}\\b`),
  `client/src/network/__tests__/protocol.test.ts v${W}`);
refusePattern(p2pProtocolTestSource, new RegExp(`\\bv${W - 1}\\b`),
  "client/src/network/__tests__/protocol.test.ts");

// The handshake pair stamps LITERALS on purpose — a frame built from
// WIRE_PROTOCOL_VERSION cannot tell a bumped client from an unbumped one — so
// the refused and the admitted number both live in this block, in the frames,
// in the title, and in the comment explaining the choice. The slice ends at
// the block's own closer, the first column-0 `});` after it, because the test
// file writes seats, object ids and timeouts as bare integers: scanning past
// the block would refuse an unrelated appended test as soon as the superseded
// number reached a value one of them already uses. Ceiling: the closer is
// found by column, so a column-0 `});` inside a template literal would end the
// slice early — the `setupFrameAt` require legs sit inside the block and red
// if that happens.
const P2P_GATE = 'describe("P2P wire-protocol version gate"';
const gateAt = p2pAdapterTestSource.indexOf(P2P_GATE);
if (gateAt < 0) {
  console.error(
    `Could not find ${P2P_GATE} in client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts: ` +
      "that block holds the only instrument that tells a bumped client from an unbumped one.",
  );
  process.exit(1);
}
const gateEnd = p2pAdapterTestSource.indexOf("\n});", gateAt);
const p2pGateBlock = p2pAdapterTestSource.slice(
  gateAt,
  gateEnd < 0 ? undefined : gateEnd,
);
const gateLabel = "client/src/adapter/__tests__/p2p-adapter-multiplayer.test.ts";
for (const n of [W - 1, W]) {
  requirePattern(p2pGateBlock, new RegExp(`setupFrameAt\\(${n}\\)`),
    `${gateLabel} setupFrameAt(${n})`);
}
// Bare as well as `v`-prefixed: the gate block writes the handshake pair as
// bare numerals, so a leftover can name the superseded version with no `v` in
// front of it. Both guards exclude identifier characters, and a dot disarms
// the match only when a digit sits on its far side — `0.<n>` and `<n>.0` are
// admitted because the digits are then part of a longer number, while a
// leading-dot decimal `.<n>`, a wildcard `<n>.x` and a sentence-ending `<n>.`
// are refused. Ceiling: an uppercase `V` prefix reads as an identifier and is
// admitted.
refusePattern(p2pGateBlock,
  new RegExp(`(?<![0-9A-Za-z_]|[0-9]\\.)v?${W - 2}(?![0-9A-Za-z_]|\\.[0-9])`),
  gateLabel);
