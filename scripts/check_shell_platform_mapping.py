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

Nothing here soft-fails to an empty set: every way of failing to read a file
raises, because an empty matrix is a subset of any mapping and a gate that
understood nothing would otherwise print a pass. The mapping is read three times
over -- `ALL`, `os_arch`, `target_triple` -- and the three must name the same
variants. `ALL` is the only thing `from_os_arch` iterates, so a variant absent
from it resolves for no platform at runtime while both matches stay exhaustive
and every population still reads as expected. Each block's `Self::` receivers are
compared as a set against the arms read out of it as well, because an arm rustfmt
broke across lines still carries its receiver and would otherwise be dropped in
silence. Two arms naming one platform, or resolving one triple, refuse for the
same reason: a variant nothing can reach.

Every comparison here is between sets of fully-qualified names, never between
their sizes. A size holds while its members are substituted underneath it, which
is the one defect each of those holes was an instance of: two variants collapsing
onto one platform, a variant absent from `ALL`, a binary attached under a name no
desktop derives.

The other direction is the same defect pointed the other way. Every variant
resolves the asset name a desktop on that platform downloads -- its triple plus
the `.exe` a `windows` variant's `executable_suffix` appends -- and `release.yml`
is what publishes those assets. A name resolved but never published is a desktop
requesting a URL that 404s. That side is read twice too, from the signing loop
and from the attached asset list, because each proves what the other cannot: the
loop's `test -s` fails the release when the binary was never built, and the asset
list is what the release actually carries. Two readings that disagree describe no
published set, so they refuse rather than pick one. Their agreement is compared
on triples, which is all the loop names; the suffix axis is held against the
mapping instead, where getting it wrong is a silent 404 rather than a release
that fails its own `test -s`.

The published set is allowed to be a superset. It quantifies over the release's
slim servers, not over desktop platforms, so a triple published for a consumer
that is not a desktop strands nothing and is not counted.

The mapping is read by regex over `ServerPlatform`'s own blocks, each anchored on
its name, so renaming a method or reshaping its arms breaks this gate loudly
rather than silently. The expected populations are checked last and carry their
own exit code: a readable set that moved is neither a coverage gap nor a failure
to read, and checking it first would let a grown matrix hide the very desktop
that cannot fetch an engine.
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

try:
    import yaml
except ModuleNotFoundError:
    # Exit 2, not 1: `main` reserves 1 for "a published platform has no mapping",
    # 2 for "I could not read this", and 3 for "a population I read perfectly has
    # moved". A missing parser is the second kind, and collapsing them would make
    # the refusal unreadable from the exit code.
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

#: `ALL` is the variant list `from_os_arch` iterates, and the two methods are the
#: arms it resolves them through. Each is anchored on its own name and bounded by
#: the closing brace at its indentation, so none of the three can be read as
#: another's contents -- the module also holds a free `target_triple()` function,
#: whose body carries no arms at all and so would read as an empty mapping rather
#: than as a wrong one.
MAPPING_ALL_BLOCK = re.compile(
    r"const ALL:\s*\[Self;\s*\d+\]\s*=\s*\[(.*?)\n    \];", re.S)
MAPPING_BLOCK = re.compile(r"fn os_arch\b[^{]*\{(.*?)\n    \}", re.S)
MAPPING_TRIPLE_BLOCK = re.compile(
    r"fn target_triple\(self\)[^{]*\{(.*?)\n    \}", re.S)

#: Every arm is read through its `Self::` receiver, which is what joins the three
#: blocks into one record per variant. The receivers are also compared as a set
#: against the arms read beside them: an arm rustfmt broke across lines still
#: carries its receiver but matches neither value pattern below, so a receiver
#: with no arm is how a dropped arm announces itself, by name.
MAPPING_RECEIVER = re.compile(r"Self::(\w+)")
MAPPING_ENTRY = re.compile(
    r'Self::(\w+)\s*=>\s*\(\s*"([^"]+)"\s*,\s*"([^"]+)"\s*\)')
MAPPING_TRIPLE_ARM = re.compile(r'Self::(\w+)\s*=>\s*"([^"]+)"')

SLIM_PREFIX = "phase-server-slim-"

#: The release job's two spellings of its published set: the `for triple in ...`
#: loop it signs, and the `phase-server-slim-*` paths it attaches. The asset
#: pattern is end-anchored so the directory half of each path, which repeats the
#: asset name, is not counted a second time; it captures the whole filename,
#: `.exe` included, because that suffix is part of the URL a desktop derives. The
#: lazy name plus the anchor is what tells a binary line from its signature.
SIGN_LOOP = re.compile(r"for triple in((?:\s*\\\s*[\w.-]+)+)\s*;\s*do")
SIGN_TRIPLE = re.compile(r"[\w.-]+")
ASSET_LINE = re.compile(rf"({SLIM_PREFIX}[\w.-]+?)(\.minisig)?$", re.M)

#: The desktop platforms a tag publishes and the mapping entries that serve them.
#: Both are expectations, not observations: a change to either is the event this
#: gate reports, so it fails and names which side moved. There is deliberately no
#: expected count for the published slim servers -- that population may exceed the
#: desktop set without stranding a desktop.
PUBLISHED_PLATFORM_COUNT = 4
MAPPED_PLATFORM_COUNT = 4


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


def _mapping_block(text: str, pattern: re.Pattern[str], what: str) -> str:
    match = pattern.search(text)
    if match is None:
        raise Refusal(f"{MAPPING_SOURCE} has no readable `ServerPlatform::{what}`; "
                      "it was renamed or reformatted, and the mapping cannot be "
                      "read")
    return match.group(1)


def _arms(block: str, pattern: re.Pattern[str],
          what: str) -> dict[str, tuple[str, ...]]:
    """One block's arms, keyed by the `Self::` receiver each is written against.

    Named, not counted: the receivers present and the arms parsed are compared as
    sets, so a dropped arm is reported by name. A count of either would hold while
    one arm was substituted for another.
    """
    arms = {match[0]: match[1:] for match in pattern.findall(block)}
    receivers = MAPPING_RECEIVER.findall(block)
    unread = sorted(set(receivers) - set(arms))
    repeated = sorted({name for name in receivers if receivers.count(name) > 1})
    if unread or repeated:
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::{what} has arms this gate could "
            f"not read. Receivers with no arm it could parse: {unread}. Receivers "
            f"written more than once: {repeated}. An arm rustfmt broke across "
            "lines still carries its receiver, so it is named here rather than "
            "dropped in silence; write it on one line, or teach this gate the "
            "shape it now takes")
    return arms


def _refuse_shared(owners: dict[str, object], what: str, why: str) -> None:
    """Refuse when two variants resolve to one value, naming the variants.

    The collision itself is the finding, so it is reported as the value and the
    variants claiming it. A population size cannot see this: two variants
    collapsing onto one value is exactly the substitution a count admits.
    """
    shared = {
        value: names
        for value in set(owners.values())
        if len(names := sorted(n for n, v in owners.items() if v == value)) > 1
    }
    if shared:
        raise Refusal(f"{MAPPING_SOURCE}: ServerPlatform::{what} resolves "
                      f"{sorted(shared.items(), key=repr)} -- those variants "
                      f"{why}")


def mapped_platforms() -> dict[str, tuple[str, str, str]]:
    """Each `ServerPlatform` variant as `(os, arch, triple)`. The desktop's authority."""
    text = _mapping_text()
    listed = MAPPING_RECEIVER.findall(
        _mapping_block(text, MAPPING_ALL_BLOCK, "ALL"))
    pairs = _arms(_mapping_block(text, MAPPING_BLOCK, "os_arch"),
                  MAPPING_ENTRY, "os_arch")
    triples = _arms(_mapping_block(text, MAPPING_TRIPLE_BLOCK, "target_triple"),
                    MAPPING_TRIPLE_ARM, "target_triple")

    if not set(listed) == set(pairs) == set(triples):
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::ALL lists {sorted(set(listed))} "
            f"while os_arch covers {sorted(pairs)} and target_triple covers "
            f"{sorted(triples)}. ALL is the only source from_os_arch iterates, so "
            "a variant missing from it resolves for no platform at runtime while "
            "both matches still compile. Add the variant to ALL; its declared "
            "length is not what this gate reads")

    mapped = {name: (*pairs[name], *triples[name]) for name in pairs}

    _refuse_shared({name: (os_name, arch)
                    for name, (os_name, arch, _) in mapped.items()},
                   "os_arch",
                   "claim the same (os, arch), so one of their triples is "
                   "unreachable and the published set would read as covered by a "
                   "mapping that cannot serve it")
    # Over the derived name, not the bare triple: the suffix is part of what a
    # desktop requests, so two triples differing only by the suffix one of their
    # platforms appends resolve to a single URL. Comparing triples sees two
    # distinct values there and lets both desktops download the same binary.
    _refuse_shared({name: slim_asset(os_name, triple)
                    for name, (os_name, _, triple) in mapped.items()},
                   "target_triple",
                   "derive one asset name, so a desktop on one of those platforms "
                   "downloads the other platform's binary and the name it should "
                   "have asked for goes unchecked here")
    return mapped


def slim_asset(os_name: str, triple: str) -> str:
    """The release asset a desktop on this platform derives and downloads.

    `native_engine.rs` builds the name from its triple plus `executable_suffix`,
    which is `.exe` under `cfg(target_os = "windows")` -- the same `windows` this
    variant's `os_arch` reports as `std::env::consts::OS`.
    """
    return f"{SLIM_PREFIX}{triple}{'.exe' if os_name == 'windows' else ''}"


def asset_triple(asset: str) -> str:
    """The triple an attached asset name carries, suffix removed."""
    return asset.removeprefix(SLIM_PREFIX).removesuffix(".exe")


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


def published_assets() -> set[str]:
    """Every `phase-server-slim-*` name the release attaches, signatures included.

    A desktop derives two URLs from its platform, the binary and the signature
    beside it, so each is its own fully-qualified identity here rather than one
    identity carrying a flag. Whichever of them a release fails to attach is then
    the name that comes back missing, instead of a pair that drops out of a count.
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
    attached = {f"{asset}{signature}" for asset, signature in lines}

    # Compared on triples, because a triple is all the loop names: it appends the
    # suffix itself, from its own test of which triple is Windows. That test being
    # wrong is self-announcing -- a Windows triple it failed to special-case fails
    # `test -s` and takes the release down with it -- whereas a wrong name in the
    # asset list publishes cleanly and 404s a desktop, which is why the suffix
    # axis is held against the mapping rather than against this loop.
    binaries = {asset_triple(asset) for asset, signature in lines if not signature}
    if signed != binaries:
        raise Refusal(
            f"{RELEASE_WORKFLOW}: {RELEASE_JOB} signs {sorted(signed)} and "
            f"attaches binaries for {sorted(binaries)}. Signed, never attached: "
            f"{sorted(signed - binaries)}. Attached, never signed: "
            f"{sorted(binaries - signed)}. A binary built and signed but never "
            "attached, or attached without ever being built, is published in "
            "neither sense this gate can report")
    return attached


def main() -> int:
    try:
        mapped = mapped_platforms()
        published = published_platforms()
        attached = published_assets()
    except Refusal as exc:
        print(f"REFUSED: {exc}", file=sys.stderr)
        return 2

    pairs = {(os_name, arch) for os_name, arch, _ in mapped.values()}
    binaries = {slim_asset(os_name, triple)
                for os_name, _, triple in mapped.values()}
    # Both URLs, because a desktop derives both and either one 404s on its own.
    resolved = {name for binary in binaries
                for name in (binary, f"{binary}.minisig")}
    status = 0

    unmapped = sorted(published - pairs)
    if unmapped:
        print(f"{SHELL_RELEASE}'s {BUILD_JOB} publishes {len(unmapped)} "
              f"platform(s) that {MAPPING_SOURCE} cannot map to an engine "
              "target:", file=sys.stderr)
        for os_name, arch in unmapped:
            print(f"  {os_name}-{arch}", file=sys.stderr)
        print("A desktop published for a platform with no ServerPlatform "
              "variant cannot download an engine. Add the variant, or stop "
              "publishing the platform.", file=sys.stderr)
        status = 1

    unpublished = sorted(resolved - attached)
    if unpublished:
        print(f"{MAPPING_SOURCE}'s ServerPlatform resolves {len(unpublished)} "
              f"asset URL(s) that {RELEASE_WORKFLOW}'s {RELEASE_JOB} job "
              "does not publish:", file=sys.stderr)
        for asset in unpublished:
            print(f"  {asset}", file=sys.stderr)
        print("A desktop resolving one of these asks the release for an asset "
              "that is not there and its download 404s. Publish that exact "
              "name, or stop resolving it.", file=sys.stderr)
        status = 1

    moved: list[str] = []
    if len(pairs) != MAPPED_PLATFORM_COUNT:
        moved.append(f"{MAPPING_SOURCE}: ServerPlatform::os_arch reads as "
                     f"{len(pairs)} platform(s), expected "
                     f"{MAPPED_PLATFORM_COUNT}: {sorted(pairs)}")
    if len(published) != PUBLISHED_PLATFORM_COUNT:
        moved.append(f"{SHELL_RELEASE}: {BUILD_JOB} publishes {len(published)} "
                     f"platform(s), expected {PUBLISHED_PLATFORM_COUNT}: "
                     f"{sorted(published)}")
    if moved:
        print("MOVED: a population this gate holds an expectation about has "
              "changed. Every check above is by name and ran clean, so this is "
              "the expectation to re-confirm, not a hole:", file=sys.stderr)
        for line in moved:
            print(f"  {line}", file=sys.stderr)
        status = status or 3

    if status:
        return status

    print(f"shell platform mapping OK: {len(published)} published platform(s) "
          f"({', '.join(f'{o}-{a}' for o, a in sorted(published))}) all mapped "
          f"by ServerPlatform ({len(mapped)} variant(s))")
    print(f"engine target triples OK: {len(binaries)} resolved asset(s) "
          f"({', '.join(sorted(binaries))}), each with its signature, all "
          f"published by {RELEASE_WORKFLOW}'s {RELEASE_JOB} job "
          f"({len(attached)} slim asset URL(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
