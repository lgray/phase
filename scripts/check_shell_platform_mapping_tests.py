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
                   triple_method: str = "target_triple",
                   listed: int | None = None,
                   wrap: int | None = None,
                   comment: int | None = None) -> str:
    """A stand-in for the real module's `ServerPlatform`.

    Variants are indexed rather than named after their platform, so a duplicate
    `(os, arch)` or a duplicate triple is expressible at all. Both methods are
    read populations, and each is also the other's overrun guard: a block regex
    that ran past one reads the other's string literals as its own entries.

    `listed` truncates `ALL` while leaving both matches exhaustive, which is the
    shape a variant added without extending `ALL` takes: it compiles, and
    `from_os_arch` reaches it for no `(os, arch)`. `wrap` renders one arm the way
    rustfmt breaks one too long for a line, in both matches. `comment` names one
    arm's variant in a legal comment, both above the arm and trailing on it, in
    both matches: the two shapes such a note takes, neither of which is an arm.
    """
    variants = [f"Platform{i}" for i in range(len(platforms))]
    in_all = variants if listed is None else variants[:listed]
    all_arms = "\n".join(f"        Self::{v}," for v in in_all)
    pairs = "\n".join(
        f'            Self::{v} => (\n                "{os_name}",\n'
        f'                "{arch}",\n            ),'
        if i == wrap else
        f'            Self::{v} => ("{os_name}", "{arch}"),'
        for i, (v, (os_name, arch, _)) in enumerate(zip(variants, platforms)))
    triples = "\n".join(
        f'            Self::{v} => {{\n                "{triple}"\n            }}'
        if i == wrap else
        f'            Self::{v} => "{triple}",'
        for i, (v, (_, _, triple)) in enumerate(zip(variants, platforms)))
    declared = "\n".join(f"    {v}," for v in variants)
    body = (f"""//! Fixture stand-in for the real native engine module.

#[derive(Clone, Copy, Debug)]
enum ServerPlatform {{
{declared}
}}

impl ServerPlatform {{
    const ALL: [Self; {len(in_all)}] = [
{all_arms}
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
    if comment is not None:
        arm = f"            Self::{variants[comment]} =>"
        note = f"// Self::{variants[comment]} is the desktop this arm serves."
        body = "\n".join(
            f"            {note}\n{line} {note}" if line.startswith(arm) else line
            for line in body.splitlines()) + "\n"
    return body


def annotate(body: str, line: str, note: str) -> str:
    """`note` spliced onto the one `line`, refusing if it is not there to annotate.

    A comment case that silently annotated nothing would pass over an
    unmarked tree, so the line it names has to be present exactly once.
    """
    if body.count(line) != 1:
        raise AssertionError(
            f"fixture carries {body.count(line)} copies of {line!r}; a comment "
            "case that annotates nothing, or two places, measures neither")
    return body.replace(line, note.replace("@", line))


#: The lines the comment cases annotate, as `mapping_source` writes them.
ALL_ENTRY = "        Self::Platform2,"
OS_ARCH_ARM = '            Self::Platform0 => ("macos", "aarch64"),'


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


def slim_asset_lines(triple: str, omit: str = "", exe: str = ".exe") -> str:
    """The binary and signature paths the release attaches for one triple.

    `omit` drops one half, which is how a triple published at only one of the
    two URLs a desktop derives from it is expressed. `exe` is the suffix a
    Windows triple's asset carries; emptying it attaches a complete pair under a
    name no Windows desktop asks for.
    """
    name = f"phase-server-slim-{triple}"
    ext = exe if "windows" in triple else ""
    halves = {"binary": f"          artifacts/{name}/{name}{ext}",
              "signature": f"          artifacts/{name}/{name}{ext}.minisig"}
    return "\n".join(line for half, line in halves.items() if half != omit)


def release_source(triples: tuple[str, ...] = DEFAULT_TRIPLES, *,
                   attached: tuple[str, ...] | None = None,
                   omit: dict[str, str] | None = None,
                   sign_step: str = "sign-release-artifacts",
                   asset_step: str = "release-assets",
                   loop_var: str = "triple",
                   exe: str = ".exe") -> str:
    """A stand-in for the release job's slim-server signing and asset steps.

    `attached` defaults to `triples`, so the two readings agree unless a case
    sets them apart. `omit` maps a triple to the half of its asset pair the
    release leaves behind.
    """
    signed = " \\\n            ".join(triples)
    files = "\n".join(slim_asset_lines(t, (omit or {}).get(t, ""), exe)
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
                      **kwargs: str | int | None) -> None:
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

    def write_workflow_text(self, body: str) -> None:
        """A workflow body the case built itself, for matrix shapes no keyword spells."""
        self._write(WORKFLOW_REL, body)

    def write_mapping_text(self, body: str) -> None:
        """A mapping body the case built itself, for shapes no keyword spells."""
        self._write(MAPPING_REL, body)

    def write_release_text(self, body: str) -> None:
        """A release body the case built itself."""
        self._write(RELEASE_REL, body)

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

    def test_a_new_published_platform_names_the_gap_and_the_moved_count(self) -> None:
        # The matrix grew past a readable mapping, so the new platform is both an
        # unmapped desktop and evidence that this gate's expected population
        # moved. The population having moved must not cost the run the name of
        # the desktop that cannot fetch an engine.
        t = self.tree()
        t.write_workflow(
            DEFAULT_PLATFORMS + (("freebsd", "x86_64", "x86_64-unknown-freebsd"),))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("freebsd-x86_64", r.stderr)
        self.assertIn("cannot map to an engine target", r.stderr)
        self.assertIn("publishes 5 platform(s)", r.stderr)

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
        # Each leg names its own reason, because "REFUSED" alone is also what an
        # interpreter with no YAML parser prints for every one of them.
        for label, prepare, reason in (
            ("absent file", lambda t: t.delete(MAPPING_REL),
             "the platform mapping is missing"),
            ("renamed method", lambda t: t.write_mapping(method="platform_pair"),
             "ServerPlatform::os_arch"),
            ("renamed triple method",
             lambda t: t.write_mapping(triple_method="server_triple"),
             "ServerPlatform::target_triple"),
        ):
            with self.subTest(mapping=label):
                t = self.tree()
                prepare(t)
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn(reason, r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_duplicate_os_arch_entry_refuses(self) -> None:
        # Five arms, two claiming linux-aarch64, so the fifth variant's triple is
        # unreachable while the distinct platforms still number the expected four.
        # The finding is which variants collided, so both are named.
        t = self.tree()
        t.write_mapping(DEFAULT_PLATFORMS + (("linux", "aarch64", "wrong-triple"),))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("('linux', 'aarch64')", r.stderr)
        self.assertIn("'Platform3', 'Platform4'", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_fifth_distinct_mapping_arm_names_its_unpublished_asset(self) -> None:
        # Five arms naming five platforms is a readable mapping whose population
        # moved, and the fifth resolves an asset the release does not carry. Both
        # are reported: the moved count explains why, the asset is what 404s.
        t = self.tree()
        t.write_mapping(
            DEFAULT_PLATFORMS + (("freebsd", "x86_64", "x86_64-unknown-freebsd"),))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("phase-server-slim-x86_64-unknown-freebsd", r.stderr)
        self.assertIn("reads as 5 platform(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_shrunken_platform_set_reports_a_moved_population(self) -> None:
        # Every population read perfectly and no desktop is stranded, so this is
        # neither a coverage gap nor a failure to read. It is the expected set
        # moving, which the exit code has to distinguish from both.
        t = self.tree()
        t.write_mapping(DEFAULT_PLATFORMS[:3])
        t.write_workflow(DEFAULT_PLATFORMS[:3])
        t.write_release(DEFAULT_TRIPLES[:3])
        r = t.run()
        self.assertEqual(r.returncode, 3, r.stdout)
        self.assertIn("reads as 3 platform(s)", r.stderr)
        self.assertIn("publishes 3 platform(s)", r.stderr)
        self.assertNotIn("REFUSED", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_variant_absent_from_all_refuses(self) -> None:
        # ALL is the only thing from_os_arch iterates, so a variant left out of
        # it resolves for no (os, arch) at runtime while both matches stay
        # exhaustive and every population still counts four.
        t = self.tree()
        t.write_mapping(listed=3)
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("ServerPlatform::ALL", r.stderr)
        self.assertIn("Platform3", r.stderr)
        # A refusal that tells the reader to adjust a number teaches them to
        # switch it off. The repair is the variant, so that is what it asks for.
        self.assertIn("Add the variant to ALL", r.stderr)
        self.assertNotIn("count", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_wrapped_match_arm_refuses(self) -> None:
        # rustfmt breaks an arm too long for one line, and an arm pattern written
        # for the single-line shape reads straight past it. Every arm carries its
        # `Self::` receiver whatever its shape, so counting those is what notices.
        t = self.tree()
        t.write_mapping(wrap=3)
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("no arm it could parse: ['Platform3']", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_comment_naming_a_variant_is_not_an_arm(self) -> None:
        # The companion of the case above, pointed the other way. A legal note
        # naming the variant it documents is not a second arm, but scanning raw
        # block text counted every mention as a receiver and refused with
        # "written more than once" -- diagnosed as an arm rustfmt broke across
        # lines, a wrapping the reader would hunt and never find.
        body = mapping_source(comment=0)
        self.assertEqual(
            body.count("// Self::Platform0"), 4,
            "the fixture must carry the note in both matches, above each arm "
            "and trailing on it, or this test passes over an unannotated tree")
        t = self.tree()
        t.write_mapping(comment=0)
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("shell platform mapping OK", r.stdout)

    def test_a_block_comment_naming_a_variant_is_not_an_arm(self) -> None:
        # The case above in Rust's other comment form, which a rule written for
        # `//` leaves untouched. Every shape it takes is covered, because an
        # enumeration of comment syntaxes admits the one it omits: trailing on
        # an arm, spanning lines with the variant in its interior, and nested --
        # the last of which `/\\*.*?\\*/` closes at the inner `*/`, leaving the
        # remainder of one comment standing as code.
        for label, note in (
            ("trailing", "@ /* Self::Platform3 ships from the other arm */"),
            ("multi-line interior", "            /* This arm serves\n"
                                    "               Self::Platform3's neighbour. */\n@"),
            ("nested", "@ /* outer /* inner */ Self::Platform3 */"),
        ):
            with self.subTest(comment=label):
                body = annotate(mapping_source(), OS_ARCH_ARM, note)
                self.assertIn("Self::Platform3", body,
                              "the comment must name a variant, or this case "
                              "passes over an unannotated tree")
                t = self.tree()
                t.write_mapping_text(body)
                r = t.run()
                self.assertEqual(r.returncode, 0, r.stderr)
                self.assertIn("shell platform mapping OK", r.stdout)

    def test_a_comment_hiding_a_variant_absent_from_all_refuses(self) -> None:
        # Why `ALL` cannot be read raw, in the direction that failed open. The
        # variant really is missing from `ALL`, so it resolves for no platform at
        # runtime, while a legal note names it. Counting that note as an entry
        # makes the three-way comparison *complete* rather than short, so the
        # gate prints a pass over the one defect it reads `ALL` at all to catch.
        for label, note in (
            ("block", "@ /* Self::Platform3 temporarily not shipped */"),
            ("line", "@ // Self::Platform3 temporarily not shipped"),
        ):
            with self.subTest(comment=label):
                t = self.tree()
                t.write_mapping_text(
                    annotate(mapping_source(listed=3), ALL_ENTRY, note))
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn("ServerPlatform::ALL", r.stderr)
                self.assertIn("Platform3", r.stderr)
                self.assertIn("Add the variant to ALL", r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_name_all_cannot_count_refuses_against_its_declared_length(self) -> None:
        # The cross-check for the one block held only by name, and the member no
        # comment rule can reach: a `Self::` inside a string literal is not a
        # comment, so it survives every removal there is. It names the variant
        # genuinely absent from `ALL`, which makes the by-name comparison agree.
        # Only rustc's own element count objects, because it is the one figure
        # here not read out of the text being checked.
        t = self.tree()
        t.write_mapping_text(annotate(mapping_source(listed=3), ALL_ENTRY,
                                      '@ "Self::Platform3",'))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("declared `[Self; 3]`", r.stderr)
        self.assertIn("read 4 entry name(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_commented_out_arm_refuses(self) -> None:
        # Removing comments before reading must not let a commented-out arm read
        # as cleanly absent. The variant is still listed in `ALL` and is still a
        # platform no arm resolves, which is a desktop that fetches no engine.
        t = self.tree()
        t.write_mapping_text(annotate(mapping_source(), OS_ARCH_ARM,
                                      "            // " + OS_ARCH_ARM.lstrip()))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("ServerPlatform::ALL", r.stderr)
        self.assertIn("Platform0", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_double_slash_inside_a_string_literal_keeps_its_arm(self) -> None:
        # The bound on comment removal: `//` inside a literal is data. Stripping
        # from it truncates the arm's value, and the arm then reads as one this
        # gate could not parse -- a refusal naming a wrapping that is not there.
        platforms = with_arch(DEFAULT_PLATFORMS, 2, "x86//64")
        t = self.tree()
        t.write_mapping(platforms)
        t.write_workflow(platforms)
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("linux-x86//64", r.stdout)

    def test_a_commented_asset_path_is_not_a_published_asset(self) -> None:
        # The same fail-open shape on the release side. `attached` is allowed to
        # be a superset, so no later comparison can object to a name added to
        # it, and a commented path names exactly the URL whose absence would
        # otherwise be reported. Both placements count: outside the heredoc,
        # where the shell reads `#` as a comment, and inside it, where `#` is
        # literal text and the line is not an attached path either way.
        triple = DEFAULT_TRIPLES[3]
        asset = f"phase-server-slim-{triple}"
        comment = f"          # artifacts/{asset}/{asset}.minisig"
        body = release_source(omit={triple: "signature"})
        for label, anchor in (("outside the heredoc", "          cat <<'EOF'"),
                              ("inside the heredoc", "          EOF")):
            with self.subTest(placement=label):
                self.assertEqual(body.count(anchor), 1,
                                 "the placement must be unambiguous, or this "
                                 "case injects somewhere it did not intend")
                t = self.tree()
                t.write_release_text(body.replace(anchor, f"{comment}\n{anchor}"))
                r = t.run()
                self.assertEqual(r.returncode, 1, r.stdout)
                self.assertIn(f"{asset}.minisig", r.stderr)
                self.assertIn("does not publish", r.stderr)

    def test_a_windows_asset_without_its_exe_suffix_fails(self) -> None:
        # A desktop derives its asset name from its triple plus the suffix its
        # own platform implies, so a Windows desktop asks for `...-msvc.exe`. A
        # release attaching a complete, signed pair under the bare triple
        # publishes nothing at either URL that desktop requests.
        t = self.tree()
        t.write_release(exe="")
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn("phase-server-slim-x86_64-pc-windows-msvc.exe", r.stderr)
        self.assertIn("does not publish", r.stderr)

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

    def test_a_published_triple_the_mapping_does_not_name_passes(self) -> None:
        # The release's slim-server set is not the desktop platform set. A triple
        # published for a consumer that is not a desktop is a superset no variant
        # resolves, and a superset cannot 404 anything a desktop asks for.
        t = self.tree()
        t.write_release(DEFAULT_TRIPLES + ("x86_64-unknown-freebsd",))
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("engine target triples OK", r.stdout)

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

    def test_each_half_of_a_derived_pair_is_its_own_identity(self) -> None:
        # A desktop derives two URLs from its platform, the binary and the
        # signature beside it, and whichever is missing 404s on its own. Each is
        # therefore its own name here: the unbuilt binary disagrees with the
        # signing loop, while the unattached signature is simply a URL nobody
        # published. Both are dropped in turn because an enumeration is falsified
        # by the member it omits.
        triple = DEFAULT_TRIPLES[3]
        for half, code, reason in (
            ("binary", 2, f"Signed, never attached: ['{triple}']"),
            ("signature", 1, f"phase-server-slim-{triple}.minisig"),
        ):
            with self.subTest(missing=half):
                t = self.tree()
                t.write_release(omit={triple: half})
                r = t.run()
                self.assertEqual(r.returncode, code, r.stdout)
                self.assertIn(reason, r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_duplicate_target_triple_arm_refuses(self) -> None:
        # Four arms resolving three triples, so one platform's desktop downloads
        # another platform's binary. The two variants sharing the triple are the
        # finding; the size of the distinct set is not.
        t = self.tree()
        t.write_mapping(with_triple(DEFAULT_PLATFORMS, 2, DEFAULT_TRIPLES[3]))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn(f"'phase-server-slim-{DEFAULT_TRIPLES[3]}'", r.stderr)
        self.assertIn("'Platform2', 'Platform3'", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_two_triples_that_derive_one_asset_name_refuse(self) -> None:
        # The other end of the same class. The suffix is part of the name a
        # desktop requests, so two variants whose triples differ only by the
        # suffix one of their platforms appends derive a single URL: the Windows
        # desktop and the linux-x86_64 desktop ask for the same file and one of
        # them gets a binary built for the other OS. Compared as bare triples
        # these are two distinct values and the tree reads as clean.
        t = self.tree()
        t.write_mapping(
            with_triple(DEFAULT_PLATFORMS, 2, f"{DEFAULT_TRIPLES[1]}.exe"))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn(f"'phase-server-slim-{DEFAULT_TRIPLES[1]}.exe'", r.stderr)
        self.assertIn("'Platform1', 'Platform2'", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_two_variants_sharing_one_bare_triple_refuse_when_one_is_windows(self) -> None:
        # The middle of that class, and the member a derived-name key alone
        # admits. The windows variant is given the linux-x86_64 variant's
        # triple, so the two derive `...-musl.exe` and `...-musl` and no asset
        # name collides at all. The release attaches both of those names, so the
        # unpublished-asset check has nothing to report and only a key on the
        # bare triple can object -- while both desktops resolve one engine build
        # and one of them is not the platform it was compiled for.
        t = self.tree()
        shared = DEFAULT_TRIPLES[2]
        t.write_mapping(with_triple(DEFAULT_PLATFORMS, 1, shared))
        t.write_release(attached=DEFAULT_TRIPLES + (f"{shared}.exe",))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn(f"'{shared}'", r.stderr)
        self.assertIn("'Platform1', 'Platform2'", r.stderr)
        # Which axis collided, not merely that something did: this is the bare
        # triple's refusal, and the derived-name one would be the wrong report.
        self.assertIn("resolve the same engine triple", r.stderr)
        self.assertNotIn("does not publish", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_an_unreadable_release_workflow_refuses(self) -> None:
        # The vacuous-green direction on the published-asset side: a set this
        # gate cannot read is empty, and an empty published set makes every
        # mapped triple unpublished or none of them, depending on which way a
        # tolerant reader took the difference. Neither answer was measured.
        for label, prepare, reason in (
            ("absent file", lambda t: t.delete(RELEASE_REL),
             "slim server assets cannot be read"),
            ("renamed signing step",
             lambda t: t.write_release(sign_step="sign-artifacts"),
             "no step id 'sign-release-artifacts'"),
            ("renamed asset step",
             lambda t: t.write_release(asset_step="assets"),
             "no step id 'release-assets'"),
            ("reshaped loop", lambda t: t.write_release(loop_var="target"),
             "`for triple in ...` loop"),
        ):
            with self.subTest(release=label):
                t = self.tree()
                prepare(t)
                r = t.run()
                self.assertEqual(r.returncode, 2, r.stdout)
                self.assertIn(reason, r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_raw_string_before_the_impl_does_not_desync_comment_removal(self) -> None:
        # Comment removal reads whole source files, so it passes through every
        # literal on the way to the blocks. A raw string read as an ordinary one
        # ends at the first quote it carries, leaving literal state open, and the
        # `//` inside it then opens a comment that swallows real code after it.
        # The raw string carries an odd number of interior quotes, which is what
        # desynchronises a reader that takes `r#"` for an ordinary literal: it
        # closes at that quote and reopens at the next one in the file, so the
        # comments below fall inside what it believes is a literal and survive
        # removal. They then read as receivers written twice. An even number
        # re-synchronises by accident and would measure nothing, and an interior
        # quote followed by `#` would close the raw string itself.
        anchor = "impl ServerPlatform {"
        body = mapping_source(comment=0)
        self.assertEqual(body.count(anchor), 1,
                         "the raw string must be spliced above the one impl, or "
                         "this case measures a tree it did not build")
        self.assertIn("// Self::Platform0", body,
                      "a comment must fall after the raw string, or the desync "
                      "has nothing to swallow and the case cannot discriminate")
        raw = 'const NOTE: &str = r#"paths contain a " character"#;\n'
        t = self.tree()
        t.write_mapping_text(body.replace(anchor, f"{raw}\n{anchor}"))
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("shell platform mapping OK", r.stdout)

    def test_a_matrix_product_axis_beside_include_refuses(self) -> None:
        # `include` is not the whole published set: a product axis publishes its
        # combinations as well, and this gate reads a platform off one entry's
        # (os, arch) pair, which a product has no entry for. Reading `include`
        # and ignoring the rest passes a tree publishing a desktop it never saw,
        # and the count guard cannot object because `include` still numbers four.
        anchor = "      matrix:\n        include:\n"
        body = workflow_source()
        self.assertEqual(body.count(anchor), 1,
                         "the axis must be spliced beside the one include, or "
                         "this case measures a shape it did not build")
        t = self.tree()
        t.write_workflow_text(body.replace(
            anchor, "      matrix:\n        os: [freebsd]\n"
                    "        arch: [x86_64]\n        include:\n"))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("declares matrix axes", r.stderr)
        self.assertIn("'arch'", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_an_asset_path_in_another_heredoc_is_not_a_published_asset(self) -> None:
        # The other half of the release-side cut: the pattern decides what a line
        # must look like, this decides where it has to be. A perfectly
        # asset-shaped path inside a different heredoc is not in the list the
        # release attaches, and counting it hides the signature genuinely
        # missing from that list -- `attached` being a superset means no later
        # comparison can object to the addition.
        triple = DEFAULT_TRIPLES[3]
        asset = f"phase-server-slim-{triple}"
        anchor = "          cat <<'EOF'"
        body = release_source(omit={triple: "signature"})
        self.assertEqual(body.count(anchor), 1,
                         "the decoy must precede the one real heredoc, or this "
                         "case measures a step it did not build")
        decoy = ("          cat <<'SKIP' > /dev/null\n"
                 f"          artifacts/{asset}/{asset}.minisig\n"
                 "          SKIP\n")
        t = self.tree()
        t.write_release_text(body.replace(anchor, decoy + anchor))
        r = t.run()
        self.assertEqual(r.returncode, 1, r.stdout)
        self.assertIn(f"{asset}.minisig", r.stderr)
        self.assertIn("does not publish", r.stderr)

    def test_every_raw_string_prefix_is_recognised(self) -> None:
        # The prefixed forms are the members an enumeration written for `r`
        # omits, and they are the ones a guard reading the character before the
        # `r` must decline: `b` and `c` are identifier characters. Only a hashed
        # form can carry an interior quote -- `r"a " b"` closes at that quote and
        # is not one literal at all -- so the desyncing body belongs to those,
        # and the hash-less forms stand for recognition alone. An odd interior
        # quote is what leaves literal state open when the opener goes
        # unrecognised; the comments below then survive removal and read as
        # receivers written twice.
        anchor = "impl ServerPlatform {"
        for prefix in ("r", "br", "cr"):
            for hashes in ("", "#"):
                with self.subTest(prefix=f"{prefix}{hashes}"):
                    body = mapping_source(comment=0)
                    self.assertEqual(body.count(anchor), 1)
                    inner = ('paths contain a " character' if hashes
                             else "paths look like this")
                    raw = (f'const NOTE: &str = '
                           f'{prefix}{hashes}"{inner}"{hashes};\n')
                    t = self.tree()
                    t.write_mapping_text(body.replace(anchor, f"{raw}\n{anchor}"))
                    r = t.run()
                    self.assertEqual(r.returncode, 0, r.stderr)
                    self.assertIn("shell platform mapping OK", r.stdout)

    def test_a_literal_carrying_the_declaration_shape_does_not_supply_the_count(self) -> None:
        # Why `ALL`'s count comes out of the same match as its entries. A string
        # literal is not a comment, so it survives removal and any second pattern
        # searching the file finds it first -- which is how a sentence of prose
        # once supplied the figure the entries are held against. The real
        # declaration says three and the entries read four, so this refuses iff
        # the count came from the declaration rather than from the literal.
        body = annotate(mapping_source(listed=3), ALL_ENTRY, '@ "Self::Platform3",')
        anchor = "impl ServerPlatform {"
        self.assertEqual(body.count(anchor), 1)
        t = self.tree()
        t.write_mapping_text(body.replace(
            anchor, 'const HELP: &str = "const ALL: [Self; 4]";\n\n' + anchor))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("declared `[Self; 3]`", r.stderr)
        self.assertIn("read 4 entry name(s)", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_second_job_publishing_desktops_refuses(self) -> None:
        # The published set is anchored on one job id, so a sibling job shipping
        # desktops is a population this gate never walks: its platforms have no
        # ServerPlatform variant checked against them, and the expected count
        # cannot object because it counts only the job that was read.
        t = self.tree()
        t.write_workflow_text(workflow_source() + """  build-shell-bsd:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        include:
          - os: freebsd
            arch: x86_64
            runner: ubuntu-latest
    steps:
      - run: echo build
""")
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("build-shell-bsd", r.stderr)
        self.assertIn("publish desktops", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_char_literal_holding_a_quote_does_not_hide_a_defect(self) -> None:
        # The last opener in Rust's literal grammar, and the only omission that
        # needs no look-behind to happen. `'"'` carries a quote that opens no
        # literal; read as code it leaves literal state open, the comment below
        # it survives removal, and the commented-out arm this tree is built
        # around stops being reported. The benign forms are controls: a char
        # literal with no quote, and a lifetime, must change nothing.
        defect = annotate(mapping_source(), OS_ARCH_ARM,
                          "            // " + OS_ARCH_ARM.lstrip())
        anchor = "impl ServerPlatform {"
        self.assertEqual(defect.count(anchor), 1)
        for label, decl, code in (
            ("char literal holding a quote", "const Q: char = '\"';", 2),
            ("byte char holding a quote", "const Q: u8 = b'\"';", 2),
            ("char literal without a quote", "const Q: u8 = b'!';", 2),
            ("a lifetime, not a literal", "struct S<'a>(&'a str);", 2),
        ):
            with self.subTest(form=label):
                t = self.tree()
                t.write_mapping_text(defect.replace(anchor, f"{decl}\n\n{anchor}"))
                r = t.run()
                self.assertEqual(r.returncode, code, r.stdout)
                self.assertIn("Platform0", r.stderr)
                self.assertNotIn("mapping OK", r.stdout)

    def test_a_second_job_publishing_desktops_by_product_axes_refuses(self) -> None:
        # A platform is an (os, arch) pair whichever shape a matrix spells it in,
        # and the axis refusal elsewhere is scoped to the job this gate reads, so
        # it never reaches a sibling. Reading only `include` entries on siblings
        # reads a subset of what publishes. The unrelated-axes case is the
        # control: a sibling matrixed on something that is not a platform must
        # not fire, or the refusal would be a tripwire on every workflow.
        for label, matrix, code in (
            ("product axes", "        os: [freebsd]\n        arch: [x86_64]", 2),
            ("unrelated axes", "        rust: [stable, beta]", 0),
        ):
            with self.subTest(sibling=label):
                t = self.tree()
                t.write_workflow_text(workflow_source() + f"""  build-shell-bsd:
    runs-on: ubuntu-latest
    strategy:
      matrix:
{matrix}
    steps:
      - run: echo build
""")
                r = t.run()
                self.assertEqual(r.returncode, code, r.stdout)

    def test_an_escaped_quote_inside_a_literal_keeps_its_arm(self) -> None:
        # The bound on the ordinary-literal reader. An escaped quote does not
        # close the literal, so a reader that stopped there would leave state
        # open across the rest of the block and swallow the comments after it.
        anchor = "impl ServerPlatform {"
        body = mapping_source(comment=0)
        self.assertEqual(body.count(anchor), 1)
        t = self.tree()
        t.write_mapping_text(body.replace(
            anchor, 'const NOTE: &str = "a \\" quote";\n\n' + anchor))
        r = t.run()
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertIn("shell platform mapping OK", r.stdout)

    def test_a_receiver_written_twice_in_one_block_refuses(self) -> None:
        # The other half of the receiver-against-arm comparison. Two arms naming
        # one variant is a variant whose second arm is unreachable, and the
        # dictionary keyed on receivers cannot see it -- the second simply
        # replaces the first. Counting the receivers is what notices.
        t = self.tree()
        t.write_mapping_text(annotate(mapping_source(), OS_ARCH_ARM,
                                      "@\n" + OS_ARCH_ARM))
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("written more than once: ['Platform0']", r.stderr)
        self.assertNotIn("mapping OK", r.stdout)

    def test_a_renamed_publishing_job_refuses(self) -> None:
        # The vacuous-green direction on the workflow side, and the leg the
        # mapping and release sides both already have: a job this gate cannot
        # find is an empty published set, and the empty set is a subset of any
        # mapping, so a tolerant reader announces a clean pass over nothing.
        # The reason is asserted, not just the refusal: a renamed job is also a
        # sibling job publishing desktops, and that refusal names `build-shell`
        # too, so a case reading only the job name passes whichever check fired.
        t = self.tree()
        t.write_workflow(job="build-desktop")
        r = t.run()
        self.assertEqual(r.returncode, 2, r.stdout)
        self.assertIn("is absent", r.stderr)
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
