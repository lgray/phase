#!/usr/bin/env python3
"""Every desktop platform the shell release publishes must have an engine mapping.

`shell-release.yml`'s `build-shell` matrix is the set of desktop platforms a tag
publishes. `native_engine.rs`'s `ServerPlatform` is how each of those desktops
resolves which engine binary to fetch. A platform published without a variant
ships a desktop that cannot acquire an engine at all.

`native_engine.rs`'s own unit test pins the pairs it expects, which is what makes
it discriminating against the mapping being deleted or broken. What it cannot see
is the matrix growing past them: nothing there refers to the workflow. This gate
is that tie, and it runs on every pull request rather than at tag time.

Both populations are counted before the subset is checked, because every way of
failing to read either file yields an empty set and an empty matrix is a subset
of any mapping -- a gate that understood nothing would print a pass. The mapping
is counted twice, as arms read and as distinct platforms: a read that finds the
wrong number of arms refuses, and so does one where two arms name the same
platform, which is the expected population of four hiding a fifth arm whose
triple nothing can reach.

The other direction is the same defect pointed the other way. Every triple
`ServerPlatform::target_triple` resolves names the `phase-server-slim-<triple>`
asset a desktop on that platform downloads, and `release.yml` is what publishes
those assets. A triple resolved but never published is a desktop requesting a URL
that 404s. That side is read twice too, from the signing loop and from the
attached asset list, because each proves what the other cannot: the loop's
`test -s` fails the release when the binary was never built, and the asset list
is what the release actually carries. Two readings that disagree describe no
published set, so they refuse rather than pick one.

The mapping is read by regex over `ServerPlatform::os_arch`'s match arms rather
than its triples, because the `(os, arch)` pair is what the matrix is compared
against and the triple is not -- so renaming that method or reshaping those arms
breaks this gate loudly rather than silently. A matrix that grows from four
platforms to five fails its count assertion for the same reason, and that is the
event this gate exists to announce: the published platform set changed, so the
mapping has to be checked against it.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

try:
    import yaml
except ModuleNotFoundError:
    # Exit 2, not 1: `main` reserves 1 for "a published platform has no mapping"
    # and 2 for "I could not read this". A missing parser is the second kind, and
    # collapsing them would make the refusal unreadable from the exit code.
    print("REFUSED: check_shell_platform_mapping: PyYAML is required and was "
          "not found; refusing to check with a weaker method", file=sys.stderr)
    sys.exit(2)

ROOT = Path(os.environ.get("SHELL_PLATFORM_MAPPING_ROOT")
            or Path(__file__).resolve().parent.parent).resolve()

MAPPING_SOURCE = "client/src-tauri/src/native_engine.rs"
SHELL_RELEASE = ".github/workflows/shell-release.yml"
BUILD_JOB = "build-shell"
RELEASE_WORKFLOW = ".github/workflows/release.yml"
RELEASE_JOB = "release"
SIGN_STEP = "sign-release-artifacts"
ASSET_STEP = "release-assets"

#: Anchored on the method name and bounded by its own closing brace, so the
#: sibling `target_triple` arms below it cannot be read as platform pairs.
MAPPING_BLOCK = re.compile(r"fn os_arch\b[^{]*\{(.*?)\n    \}", re.S)
MAPPING_ENTRY = re.compile(r'=>\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)')

#: Anchored on the method's receiver: the module also holds a free
#: `target_triple()` function, whose body carries no platform arms at all and so
#: would read as an empty mapping rather than as a wrong one.
MAPPING_TRIPLE_BLOCK = re.compile(
    r"fn target_triple\(self\)[^{]*\{(.*?)\n    \}", re.S)
MAPPING_TRIPLE_ARM = re.compile(r'=>\s*"([^"]+)"')

#: The release job's two spellings of its published set: the `for triple in ...`
#: loop it signs, and the `phase-server-slim-<triple>` paths it attaches. The
#: asset pattern is end-anchored so the directory half of each path, which
#: repeats the asset name, is not counted a second time. A triple carries no
#: `.`, so neither optional suffix can be absorbed into the captured name and
#: the trailing group is what tells a binary line from its signature.
SIGN_LOOP = re.compile(r"for triple in((?:\s*\\\s*[\w.-]+)+)\s*;\s*do")
SIGN_TRIPLE = re.compile(r"[\w.-]+")
ASSET_LINE = re.compile(
    r"phase-server-slim-([\w-]+?)(?:\.exe)?(\.minisig)?$", re.M)

#: The desktop platforms a tag publishes, the mapping entries that serve them,
#: and the slim server assets the release carries for them. All are expectations,
#: not observations: a change to any is the event this gate reports, so it fails
#: and names which side moved.
PUBLISHED_PLATFORM_COUNT = 4
MAPPED_PLATFORM_COUNT = 4
PUBLISHED_TRIPLE_COUNT = 4


class Refusal(Exception):
    """A file could not be read the way this gate needs to read it.

    The single authority for every degraded read. No extractor soft-fails by
    returning an empty set, because an empty set passes the subset check.
    """


def _mapping_text() -> str:
    """`native_engine.rs`'s source, or a refusal if it is not there to read."""
    path = ROOT / MAPPING_SOURCE
    if not path.is_file():
        raise Refusal(f"{MAPPING_SOURCE} does not exist; the platform mapping "
                      "is missing, so no published platform can be checked "
                      "against it")
    return path.read_text(encoding="utf-8")


def mapped_platforms() -> set[tuple[str, str]]:
    """The `(os, arch)` pairs `ServerPlatform` resolves. The desktop's authority."""
    block = MAPPING_BLOCK.search(_mapping_text())
    if block is None:
        raise Refusal(f"{MAPPING_SOURCE} has no readable `ServerPlatform::"
                      "os_arch` match; it was renamed or reformatted, and the "
                      "mapping cannot be read")

    entries = MAPPING_ENTRY.findall(block.group(1))
    platforms = set(entries)
    if len(platforms) != len(entries):
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::os_arch reads {len(entries)} "
            f"arm(s) naming only {len(platforms)} distinct platform(s): "
            f"{sorted(entries)}. Two variants claim the same (os, arch), so one "
            "of their triples is unreachable and the published set would read as "
            "covered by a mapping that cannot serve it")
    if len(platforms) != MAPPED_PLATFORM_COUNT:
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::os_arch reads as "
            f"{len(platforms)} platform(s), expected {MAPPED_PLATFORM_COUNT}: "
            f"{sorted(platforms)}. Either the mapping changed -- check it against "
            f"{SHELL_RELEASE}'s {BUILD_JOB} matrix and update the expected "
            "count -- or its arms no longer match the shape this gate reads")
    return platforms


def mapped_triples() -> set[str]:
    """The target triples `ServerPlatform` resolves, one release asset each."""
    block = MAPPING_TRIPLE_BLOCK.search(_mapping_text())
    if block is None:
        raise Refusal(f"{MAPPING_SOURCE} has no readable `ServerPlatform::"
                      "target_triple` match; it was renamed or reformatted, and "
                      "the assets the desktop asks for cannot be read")

    arms = MAPPING_TRIPLE_ARM.findall(block.group(1))
    triples = set(arms)
    if len(triples) != len(arms):
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::target_triple reads {len(arms)} "
            f"arm(s) resolving only {len(triples)} distinct triple(s): "
            f"{sorted(arms)}. Two variants resolve one triple, so a desktop on "
            "one of those platforms downloads the other platform's binary and "
            "the triple it should have asked for goes unchecked here")
    if len(triples) != MAPPED_PLATFORM_COUNT:
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::target_triple resolves "
            f"{len(triples)} triple(s), expected {MAPPED_PLATFORM_COUNT}: "
            f"{sorted(triples)}. Either the mapping changed -- check every "
            f"triple against {RELEASE_WORKFLOW}'s {RELEASE_JOB} job and update "
            "the expected count -- or its arms no longer match the shape this "
            "gate reads")
    return triples


def published_platforms() -> set[tuple[str, str]]:
    """The `(os, arch)` pairs `build-shell` publishes a desktop for."""
    path = ROOT / SHELL_RELEASE
    if not path.is_file():
        raise Refusal(f"{SHELL_RELEASE} does not exist; the published platform "
                      "set cannot be read")
    try:
        workflow = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise Refusal(f"{SHELL_RELEASE} is not parseable YAML: {exc}") from exc

    job = ((workflow or {}).get("jobs") or {}).get(BUILD_JOB)
    if not isinstance(job, dict):
        raise Refusal(f"{SHELL_RELEASE}: job '{BUILD_JOB}' is absent; the job "
                      "that publishes desktop packages was renamed, and this "
                      "gate no longer knows which platforms ship")

    include = (job.get("strategy") or {}).get("matrix", {})
    include = include.get("include") if isinstance(include, dict) else None
    if not isinstance(include, list):
        raise Refusal(f"{SHELL_RELEASE}: {BUILD_JOB} declares no "
                      "strategy.matrix.include list; the published platform set "
                      "cannot be read from this shape")

    platforms: set[tuple[str, str]] = set()
    for entry in include:
        if not isinstance(entry, dict) or not {"os", "arch"} <= entry.keys():
            raise Refusal(f"{SHELL_RELEASE}: {BUILD_JOB} matrix has an entry "
                          f"without both `os` and `arch`: {entry!r}. This gate "
                          "identifies a platform by that pair")
        platforms.add((str(entry["os"]), str(entry["arch"])))

    if len(platforms) != PUBLISHED_PLATFORM_COUNT:
        raise Refusal(
            f"{SHELL_RELEASE}: {BUILD_JOB} publishes {len(platforms)} "
            f"platform(s), expected {PUBLISHED_PLATFORM_COUNT}: "
            f"{sorted(platforms)}. If the matrix gained or lost a platform, "
            f"check {MAPPING_SOURCE}'s ServerPlatform covers the new set and "
            "update the expected count with it")
    return platforms


def _release_step_bodies() -> dict[str, str]:
    """The shell body of each identified step in the release-publishing job."""
    path = ROOT / RELEASE_WORKFLOW
    if not path.is_file():
        raise Refusal(f"{RELEASE_WORKFLOW} does not exist; the set of published "
                      "slim server assets cannot be read")
    try:
        workflow = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise Refusal(f"{RELEASE_WORKFLOW} is not parseable YAML: {exc}") from exc

    job = ((workflow or {}).get("jobs") or {}).get(RELEASE_JOB)
    steps = job.get("steps") if isinstance(job, dict) else None
    if not isinstance(steps, list):
        raise Refusal(f"{RELEASE_WORKFLOW}: job '{RELEASE_JOB}' declares no "
                      "steps list; the job that publishes the slim server "
                      "binaries was renamed or reshaped, and this gate no "
                      "longer knows which triples ship")
    return {step["id"]: str(step.get("run") or "")
            for step in steps
            if isinstance(step, dict) and isinstance(step.get("id"), str)}


def _step_body(bodies: dict[str, str], step_id: str, reads: str) -> str:
    body = bodies.get(step_id)
    if body is None:
        raise Refusal(f"{RELEASE_WORKFLOW}: {RELEASE_JOB} has no step id "
                      f"'{step_id}', so {reads} cannot be read")
    return body


def published_triples() -> set[str]:
    """The triples published as `phase-server-slim-<triple>` release assets.

    Published means every URL a desktop derives from its triple is there: the
    release signs it, attaches the binary, and attaches the signature beside it.
    A triple carrying only part of that is a 404 on the part that is missing, so
    it does not enter this set and the readings disagree instead.
    """
    bodies = _release_step_bodies()

    loop = SIGN_LOOP.search(
        _step_body(bodies, SIGN_STEP, "the triples the release signs"))
    if loop is None:
        raise Refusal(f"{RELEASE_WORKFLOW}: {SIGN_STEP} has no readable `for "
                      "triple in ...` loop; the signed set was reshaped, and a "
                      "set this gate cannot read is not an empty one")
    signed = set(SIGN_TRIPLE.findall(loop.group(1)))
    lines = ASSET_LINE.findall(
        _step_body(bodies, ASSET_STEP, "the assets the release attaches"))
    attached = ({triple for triple, signature in lines if not signature}
                & {triple for triple, signature in lines if signature})

    if signed != attached:
        half = sorted({triple for triple, _ in lines} - attached)
        raise Refusal(
            f"{RELEASE_WORKFLOW}: {RELEASE_JOB} signs {len(signed)} triple(s) "
            f"and attaches assets for {len(attached)}. Signed, never attached: "
            f"{sorted(signed - attached)}. Attached, never signed: "
            f"{sorted(attached - signed)}. Attached at only one of the two URLs "
            f"a desktop derives from a triple: {half}. A binary built and signed "
            "but never attached, or attached without the signature the desktop "
            "verifies, is published in neither sense this gate can report")
    if len(signed) != PUBLISHED_TRIPLE_COUNT:
        raise Refusal(
            f"{RELEASE_WORKFLOW}: {RELEASE_JOB} publishes {len(signed)} slim "
            f"server asset(s), expected {PUBLISHED_TRIPLE_COUNT}: "
            f"{sorted(signed)}. If the release gained or lost a triple, check "
            f"{MAPPING_SOURCE}'s ServerPlatform against the new set and update "
            "the expected count with it")
    return signed


def main() -> int:
    try:
        mapped = mapped_platforms()
        published = published_platforms()
        triples = mapped_triples()
        published_assets = published_triples()
    except Refusal as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return 2

    unmapped = sorted(published - mapped)
    if unmapped:
        print(f"{SHELL_RELEASE}'s {BUILD_JOB} publishes {len(unmapped)} "
              f"platform(s) that {MAPPING_SOURCE} cannot map to an engine "
              "target:", file=sys.stderr)
        for os_name, arch in unmapped:
            print(f"  {os_name}-{arch}", file=sys.stderr)
        print("A desktop published for a platform with no ServerPlatform "
              "variant cannot download an engine. Add the variant, or stop "
              "publishing the platform.", file=sys.stderr)
        return 1

    unpublished = sorted(triples - published_assets)
    if unpublished:
        print(f"{MAPPING_SOURCE}'s ServerPlatform resolves {len(unpublished)} "
              f"target triple(s) that {RELEASE_WORKFLOW}'s {RELEASE_JOB} job "
              "does not publish as a release asset:", file=sys.stderr)
        for triple in unpublished:
            print(f"  phase-server-slim-{triple}", file=sys.stderr)
        print("A desktop resolving one of these asks the release for an asset "
              "that is not there and its download 404s. Publish the triple, or "
              "stop resolving it.", file=sys.stderr)
        return 1

    print(f"shell platform mapping OK: {len(published)} published platform(s) "
          f"({', '.join(f'{o}-{a}' for o, a in sorted(published))}) all mapped "
          f"by ServerPlatform ({len(mapped)} entr(y/ies))")
    print(f"engine target triples OK: {len(triples)} resolved triple(s) "
          f"({', '.join(sorted(triples))}) all published by {RELEASE_WORKFLOW}'s "
          f"{RELEASE_JOB} job ({len(published_assets)} slim asset(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
