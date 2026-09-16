#!/usr/bin/env python3
"""Tests for check_shell_platform_mapping.py.

The gate compares populations it reads out of three files, and every way of
failing to read one yields an empty set -- which is a subset of any mapping. So
what matters is not that the gate passes a correct tree but that it refuses an
unreadable one instead of printing a pass over nothing. Each case builds a
throwaway tree holding all three files and points the real script at it through
SHELL_PLATFORM_MAPPING_ROOT.

Every fixture materialises all three files even when the case under test concerns
only one of them. A partial tree would make the gate refuse for a reason the test
did not intend, and a refusal that arrives for the wrong reason proves nothing
about the property being tested.
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).resolve().parent / "check_shell_platform_mapping.py"
MAPPING_REL = "client/src-tauri/src/native_engine.rs"
WORKFLOW_REL = ".github/workflows/shell-release.yml"
RELEASE_REL = ".github/workflows/release.yml"
REAL_MAPPING = Path(__file__).resolve().parent.parent / MAPPING_REL
REAL_WORKFLOW = Path(__file__).resolve().parent.parent / WORKFLOW_REL
REAL_RELEASE = Path(__file__).resolve().parent.parent / RELEASE_REL

#: The four published platforms and their triples, as all three files carry them.
DEFAULT_PLATFORMS = (
    ("macos", "aarch64", "aarch64-apple-darwin"),
    ("windows", "x86_64", "x86_64-pc-windows-msvc"),
    ("linux", "x86_64", "x86_64-unknown-linux-musl"),
    ("linux", "aarch64", "aarch64-unknown-linux-musl"),
)
DEFAULT_TRIPLES = tuple(triple for _, _, triple in DEFAULT_PLATFORMS)
# Named in a fixture and nowhere in the real tree, so a checker that read this
# checkout instead of the fixture could not produce them.
SENTINEL_ARCH = "fixture-sentinel-arch"
SENTINEL_TRIPLE = "fixture-sentinel-triple"

Platforms = tuple[tuple[str, str, str], ...]


def with_arch(platforms: Platforms, index: int, arch: str) -> Platforms:
    """One platform's arch replaced, keeping the population size."""
    out = list(platforms)
    os_name, _, triple = out[index]
    out[index] = (os_name, arch, triple)
    return tuple(out)


def with_triple(platforms: Platforms, index: int, triple: str) -> Platforms:
    """One platform's triple replaced, keeping the population size."""
    out = list(platforms)
    os_name, arch, _ = out[index]
    out[index] = (os_name, arch, triple)
    return tuple(out)


def mapping_source(platforms: Platforms = DEFAULT_PLATFORMS, *,
                   method: str = "os_arch",
                   triple_method: str = "target_triple") -> str:
    """A stand-in for the real module's `ServerPlatform`.

    Variants are indexed rather than named after their platform, so a duplicate
    `(os, arch)` or a duplicate triple is expressible at all. Both methods are
    read populations, and each is also the other's overrun guard: a block regex
    that ran past one reads the other's string literals as its own entries.
    """
    variants = [f"Platform{i}" for i in range(len(platforms))]
    listed = "\n".join(f"        Self::{v}," for v in variants)
    pairs = "\n".join(
        f'            Self::{v} => ("{os_name}", "{arch}"),'
        for v, (os_name, arch, _) in zip(variants, platforms))
    triples = "\n".join(
        f'            Self::{v} => "{triple}",'
        for v, (_, _, triple) in zip(variants, platforms))
    declared = "\n".join(f"    {v}," for v in variants)
    return (f"""//! Fixture stand-in for the real native engine module.

#[derive(Clone, Copy, Debug)]
enum ServerPlatform {{
{declared}
}}

impl ServerPlatform {{
    const ALL: [Self; {len(platforms)}] = [
{listed}
    ];

    fn {method}(self) -> (&'static str, &'static str) {{
        match self {{
{pairs}
        }}
    }}

    fn {triple_method}(self) -> &'static str {{
        match self {{
{triples}
        }}
    }}
}}
""")


def workflow_source(platforms: Platforms = DEFAULT_PLATFORMS, *,
                    job: str = "build-shell", os_key: str = "os") -> str:
    include = "\n".join(
        f"          - {os_key}: {os_name}\n"
        f"            arch: {arch}\n"
        f"            runner: ubuntu-latest"
        for os_name, arch, _ in platforms)
    return f"""name: Shell release
on:
  push:
    tags: ['shell-v*']
jobs:
  {job}:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
{include}
    steps:
      - run: echo build
"""


def slim_asset_lines(triple: str, omit: str = "") -> str:
    """The binary and signature paths the release attaches for one triple.

    `omit` drops one half, which is how a triple published at only one of the
    two URLs a desktop derives from it is expressed.
    """
    name = f"phase-server-slim-{triple}"
    ext = ".exe" if "windows" in triple else ""
    halves = {"binary": f"          artifacts/{name}/{name}{ext}",
              "signature": f"          artifacts/{name}/{name}{ext}.minisig"}
    return "\n".join(line for half, line in halves.items() if half != omit)


def release_source(triples: tuple[str, ...] = DEFAULT_TRIPLES, *,
                   attached: tuple[str, ...] | None = None,
                   omit: dict[str, str] | None = None,
                   sign_step: str = "sign-release-artifacts",
                   asset_step: str = "release-assets",
                   loop_var: str = "triple") -> str:
    """A stand-in for the release job's slim-server signing and asset steps.

    `attached` defaults to `triples`, so the two readings agree unless a case
    sets them apart. `omit` maps a triple to the half of its asset pair the
    release leaves behind.
    """
    signed = " \\\n            ".join(triples)
    files = "\n".join(slim_asset_lines(t, (omit or {}).get(t, ""))
                      for t in (triples if attached is None else attached))
    return f"""name: Release
on:
  push:
    tags: ['v*']
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - name: Sign slim server artifacts
        id: {sign_step}
        run: |
          for {loop_var} in \\
            {signed}; do
            binary="artifacts/phase-server-slim-${loop_var}/phase-server-slim-${loop_var}"
            test -s "$binary"
            sign "$binary"
          done

      - name: Build release asset list
        id: {asset_step}
        run: |
          cat <<'EOF'
{files}
          EOF
"""


class MappingTree:
    """A throwaway tree holding every file the gate reads."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.write_mapping()
        self.write_workflow()
        self.write_release()

    def _write(self, rel: str, body: str) -> None:
        path = self.root / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(body, encoding="utf-8")

    def write_mapping(self, platforms: Platforms = DEFAULT_PLATFORMS,
                      **kwargs: str) -> None:
        self._write(MAPPING_REL, mapping_source(platforms, **kwargs))

    def write_workflow(self, platforms: Platforms = DEFAULT_PLATFORMS,
                       **kwargs: str) -> None:
        self._write(WORKFLOW_REL, workflow_source(platforms, **kwargs))

    def write_release(self, triples: tuple[str, ...] = DEFAULT_TRIPLES, *,
                      attached: tuple[str, ...] | None = None,
                      omit: dict[str, str] | None = None,
                      **kwargs: str) -> None:
        self._write(RELEASE_REL, release_source(triples, attached=attached,
                                                omit=omit, **kwargs))

    def delete(self, rel: str) -> None:
        (self.root / rel).unlink()

    def run(self) -> subprocess.CompletedProcess:
        return subprocess.run(
            [sys.executable, str(SCRIPT)],
            env={**os.environ, "SHELL_PLATFORM_MAPPING_ROOT": str(self.root)},
            capture_output=True, text=True,
        )


class ShellPlatformMappingTests(unittest.TestCase):
    def tree(self) -> MappingTree:
        d = tempfile.mkdtemp()
        self.addCleanup(shutil.rmtree, d, ignore_errors=True)
        return MappingTree(Path(d))

    def test_a_complete_tree_passes(self) -> None:
        # Without this a gate that refuses everything would look identical to a
        # gate that works.
        t = self.tree()
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("shell platform mapping OK", r.stdout)
        self.assertIn("engine target triples OK", r.stdout)
        for triple in DEFAULT_TRIPLES:
            self.assertIn(triple, r.stdout)

    def test_an_unmapped_published_platform_fails(self) -> None:
        # Four platforms on each side, so neither count assertion fires and only
        # the subset check can catch it.
        t = self.tree()
        t.write_workflow(with_arch(DEFAULT_PLATFORMS, 3, SENTINEL_ARCH))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(SENTINEL_ARCH, r.stderr)
        self.assertIn("cannot map to an engine target", r.stderr)

    def test_a_new_published_platform_refuses(self) -> None:
        # The mapping is readable and the matrix grew past it. Reporting this as
        # a coverage gap would be true but weaker than what it is: the set this
        # gate's expectations describe has changed.
        t = self.tree()
        t.write_workflow(
            DEFAULT_PLATFORMS + (("freebsd", "x86_64", "x86_64-unknown-freebsd"),))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("publishes 5 platform(s)", r.stderr)
        self.assertIn("REFUSED", r.stderr)

    def test_a_renamed_matrix_key_refuses(self) -> None:
        # The vacuous-green direction: a matrix this gate cannot read is an empty
        # published set, and the empty set is a subset of any mapping, so a
        # tolerant reader would announce a clean pass over nothing.
        t = self.tree()
        t.write_workflow(os_key="platform")
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("without both `os` and `arch`", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_an_unreadable_mapping_refuses(self) -> None:
        # Both ways the authority goes unread. An absent mapping and a renamed
        # accessor are the same empty set, and it maps no published platform.
        for label, prepare in (
            ("absent file", lambda t: t.delete(MAPPING_REL)),
            ("renamed method", lambda t: t.write_mapping(method="platform_pair")),
            ("renamed triple method",
             lambda t: t.write_mapping(triple_method="server_triple")),
        ):
            with self.subTest(mapping=label):
                t = self.tree()
                prepare(t)
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn("REFUSED", r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_duplicate_os_arch_entry_refuses(self) -> None:
        # Five arms, two claiming linux-aarch64: counting distinct keys alone
        # reads this as the expected population of four and exits 0, while the
        # fifth variant's triple is unreachable. Both counts are checked for
        # this reason.
        t = self.tree()
        t.write_mapping(DEFAULT_PLATFORMS + (("linux", "aarch64", "wrong-triple"),))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("5 arm(s) naming only 4 distinct platform(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_fifth_distinct_mapping_arm_refuses(self) -> None:
        # The other half of the two counts: five arms naming five platforms is a
        # readable mapping whose population moved, and the matrix has to be
        # checked against the new one rather than credited with covering it.
        t = self.tree()
        t.write_mapping(
            DEFAULT_PLATFORMS + (("freebsd", "x86_64", "x86_64-unknown-freebsd"),))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("reads as 5 platform(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_mapped_triple_the_release_does_not_publish_fails(self) -> None:
        # Four triples on each side, so neither count assertion fires and only
        # the subset check can catch it. A desktop on this platform resolves an
        # asset name the release never carries and its download 404s.
        t = self.tree()
        t.write_mapping(with_triple(DEFAULT_PLATFORMS, 3, SENTINEL_TRIPLE))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(SENTINEL_TRIPLE, r.stderr)
        self.assertIn("does not publish", r.stderr)

    def test_a_published_triple_the_mapping_does_not_name_refuses(self) -> None:
        # An asset no variant resolves breaks no desktop, so this is not a
        # coverage gap in the direction above. It is the published set moving
        # out from under this gate's expectations, which is the event a grown
        # build-shell matrix reports, and it refuses for the same reason.
        t = self.tree()
        t.write_release(DEFAULT_TRIPLES + ("x86_64-unknown-freebsd",))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("publishes 5 slim server asset(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_signed_triple_the_release_never_attaches_refuses(self) -> None:
        # The signing loop and the asset list are two readings of one published
        # set. A triple signed but never attached is a built, verified binary at
        # a URL that 404s, and a reader trusting the loop alone passes it.
        t = self.tree()
        t.write_release(attached=DEFAULT_TRIPLES[:3])
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("signs", r.stderr)
        self.assertIn(DEFAULT_TRIPLES[3], r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_half_attached_triple_refuses(self) -> None:
        # A desktop derives two URLs from its triple, the binary and the
        # signature beside it. A release carrying one of them 404s on the other,
        # and an asset list read for the triple's name alone counts the pair as
        # published from either line. Both halves are dropped in turn because an
        # enumeration is falsified by the member it omits.
        for half in ("binary", "signature"):
            with self.subTest(missing=half):
                t = self.tree()
                t.write_release(omit={DEFAULT_TRIPLES[3]: half})
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn(DEFAULT_TRIPLES[3], r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_duplicate_target_triple_arm_refuses(self) -> None:
        # Four arms resolving three triples: counting distinct triples alone
        # reads this as the expected population of four and exits 0, while one
        # platform's desktop downloads another platform's binary.
        t = self.tree()
        t.write_mapping(with_triple(DEFAULT_PLATFORMS, 2, DEFAULT_TRIPLES[3]))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("4 arm(s) resolving only 3 distinct triple(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_an_unreadable_release_workflow_refuses(self) -> None:
        # The vacuous-green direction on the published-asset side: a set this
        # gate cannot read is empty, and an empty published set makes every
        # mapped triple unpublished or none of them, depending on which way a
        # tolerant reader took the difference. Neither answer was measured.
        for label, prepare in (
            ("absent file", lambda t: t.delete(RELEASE_REL)),
            ("renamed signing step",
             lambda t: t.write_release(sign_step="sign-artifacts")),
            ("renamed asset step",
             lambda t: t.write_release(asset_step="assets")),
            ("reshaped loop", lambda t: t.write_release(loop_var="target")),
        ):
            with self.subTest(release=label):
                t = self.tree()
                prepare(t)
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn("REFUSED", r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_the_harness_reads_the_fixture_not_the_real_tree(self) -> None:
        # If SHELL_PLATFORM_MAPPING_ROOT were ignored, every case above would be
        # measuring this checkout and the passing ones would be vacuous.
        for real in (REAL_MAPPING, REAL_WORKFLOW, REAL_RELEASE):
            body = real.read_text(encoding="utf-8")
            for sentinel in (SENTINEL_ARCH, SENTINEL_TRIPLE):
                self.assertNotIn(sentinel, body,
                                 "the sentinel must not exist in the real tree, "
                                 "or this test proves nothing")
        t = self.tree()
        t.write_workflow(with_arch(DEFAULT_PLATFORMS, 2, SENTINEL_ARCH))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(SENTINEL_ARCH, r.stderr)

        t = self.tree()
        t.write_mapping(with_triple(DEFAULT_PLATFORMS, 2, SENTINEL_TRIPLE))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(SENTINEL_TRIPLE, r.stderr)


if __name__ == "__main__":
    unittest.main()
