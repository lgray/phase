#!/usr/bin/env python3
"""Tests for check_shell_platform_mapping.py.

The gate compares two populations it reads out of two files, and every way of
failing to read either one yields an empty set -- which is a subset of any
mapping. So what matters is not that the gate passes a correct tree but that it
refuses an unreadable one instead of printing a pass over nothing. Each case
builds a throwaway tree holding both files and points the real script at it
through SHELL_PLATFORM_MAPPING_ROOT.

Every fixture materialises both files even when the case under test concerns only
one of them. A partial tree would make the gate refuse for a reason the test did
not intend, and a refusal that arrives for the wrong reason proves nothing about
the property being tested.
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
REAL_MAPPING = Path(__file__).resolve().parent.parent / MAPPING_REL
REAL_WORKFLOW = Path(__file__).resolve().parent.parent / WORKFLOW_REL

#: The four published platforms and their triples, as both files carry them.
DEFAULT_PLATFORMS = (
    ("macos", "aarch64", "aarch64-apple-darwin"),
    ("windows", "x86_64", "x86_64-pc-windows-msvc"),
    ("linux", "x86_64", "x86_64-unknown-linux-musl"),
    ("linux", "aarch64", "aarch64-unknown-linux-musl"),
)
# Named in a fixture and nowhere in the real tree, so a checker that read this
# checkout instead of the fixture could not produce it.
SENTINEL_ARCH = "fixture-sentinel-arch"

Platforms = tuple[tuple[str, str, str], ...]


def with_arch(platforms: Platforms, index: int, arch: str) -> Platforms:
    """One platform's arch replaced, keeping the population size."""
    out = list(platforms)
    os_name, _, triple = out[index]
    out[index] = (os_name, arch, triple)
    return tuple(out)


def mapping_source(platforms: Platforms = DEFAULT_PLATFORMS, *,
                   method: str = "os_arch") -> str:
    """A stand-in for the real module's `ServerPlatform`.

    Variants are indexed rather than named after their platform, so a duplicate
    `(os, arch)` is expressible at all. The `target_triple` arms are here because
    they hold string literals too: a block regex that overran `os_arch` would
    read them as platform pairs.
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

    fn target_triple(self) -> &'static str {{
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


class MappingTree:
    """A throwaway tree holding both files the gate reads."""

    def __init__(self, root: Path) -> None:
        self.root = root
        self.write_mapping()
        self.write_workflow()

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

    def test_the_harness_reads_the_fixture_not_the_real_tree(self) -> None:
        # If SHELL_PLATFORM_MAPPING_ROOT were ignored, every case above would be
        # measuring this checkout and the passing ones would be vacuous.
        for real in (REAL_MAPPING, REAL_WORKFLOW):
            self.assertNotIn(SENTINEL_ARCH, real.read_text(encoding="utf-8"),
                             "the sentinel must not exist in the real tree, or "
                             "this test proves nothing")
        t = self.tree()
        t.write_workflow(with_arch(DEFAULT_PLATFORMS, 2, SENTINEL_ARCH))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(SENTINEL_ARCH, r.stderr)


if __name__ == "__main__":
    unittest.main()
