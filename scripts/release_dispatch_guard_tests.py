#!/usr/bin/env python3
"""Regression tests for release.yml's recovery-dispatch guard.

The guard is release authorization: it decides whether a `workflow_dispatch`
run may publish the tag it was asked for. Its shell is read out of the
committed workflow rather than copied here, so an edit that weakens the guard
fails these tests instead of leaving them passing against a stale duplicate.
"""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

WORKFLOW = Path(__file__).resolve().parent.parent / ".github/workflows/release.yml"
STEP_NAME = "Require a tag ref for recovery dispatches"


class GuardStepNotFound(Exception):
    pass


def _step_lines(lines: list[str], name: str) -> list[str]:
    """Lines of the step whose `- name:` is `name`, up to the next sibling step."""
    start = None
    for i, line in enumerate(lines):
        if line.strip() == f"- name: {name}":
            start = i
            break
    if start is None:
        raise GuardStepNotFound(f"no step named {name!r} in {WORKFLOW}")
    indent = len(lines[start]) - len(lines[start].lstrip())
    end = len(lines)
    for i in range(start + 1, len(lines)):
        stripped = lines[i].strip()
        if not stripped or stripped.startswith("#"):
            continue
        cur = len(lines[i]) - len(lines[i].lstrip())
        if cur <= indent and stripped.startswith("- "):
            end = i
            break
    return lines[start:end]


def guard_step() -> tuple[str, list[str]]:
    """The guard's shell body and the rest of its step lines."""
    lines = WORKFLOW.read_text(encoding="utf-8").splitlines()
    step = _step_lines(lines, STEP_NAME)
    run_at = next((i for i, l in enumerate(step) if l.strip() == "run: |"), None)
    if run_at is None:
        raise GuardStepNotFound(f"step {STEP_NAME!r} has no `run: |` block")
    body_indent = len(step[run_at]) - len(step[run_at].lstrip()) + 2
    body = [l[body_indent:] if len(l) > body_indent else "" for l in step[run_at + 1 :]]
    return "\n".join(body), step[:run_at]


def dispatch(ref_type: str, ref_name: str, input_tag: str) -> subprocess.CompletedProcess:
    body, _ = guard_step()
    return subprocess.run(
        ["bash", "-c", body],
        env={"PATH": "/usr/bin:/bin", "REF_TYPE": ref_type, "REF_NAME": ref_name,
             "INPUT_TAG": input_tag},
        capture_output=True, text=True,
    )


class RecoveryDispatchGuardTests(unittest.TestCase):
    def test_branch_ref_is_rejected(self) -> None:
        r = dispatch("branch", "main", "v0.72.0")
        self.assertEqual(r.returncode, 1)
        self.assertIn("::error::", r.stdout)
        self.assertIn("not from branch 'main'", r.stdout)

    def test_tag_ref_releasing_a_different_tag_is_rejected(self) -> None:
        # The environment admits any policy-matching tag, but the steps below
        # release `inputs.tag`. Without this the run publishes v0.72.0 from a
        # run whose ref says v0.71.0.
        r = dispatch("tag", "v0.71.0", "v0.72.0")
        self.assertEqual(r.returncode, 1)
        self.assertIn("::error::", r.stdout)
        self.assertIn("v0.71.0", r.stdout)
        self.assertIn("v0.72.0", r.stdout)

    def test_tag_ref_matching_the_requested_release_is_accepted(self) -> None:
        r = dispatch("tag", "v0.72.0", "v0.72.0")
        self.assertEqual(r.returncode, 0, r.stderr)
        self.assertNotIn("::error::", r.stdout)

    def test_the_two_rejections_are_distinguishable(self) -> None:
        # Distinct messages, so neither failure mode can hide behind the other
        # and an operator learns which precondition they tripped.
        branch = dispatch("branch", "main", "v0.72.0").stdout
        mismatch = dispatch("tag", "v0.71.0", "v0.72.0").stdout
        self.assertNotEqual(branch, mismatch)

    def test_guard_is_wired_to_the_dispatch_path(self) -> None:
        # Without this the shell tests above could pass while the step no longer
        # runs, or no longer receives the values it branches on.
        _, header = guard_step()
        header_text = "\n".join(header)
        self.assertIn("if: github.event_name == 'workflow_dispatch'", header_text)
        for var, expr in (
            ("REF_TYPE", "github.ref_type"),
            ("REF_NAME", "github.ref_name"),
            ("INPUT_TAG", "inputs.tag"),
        ):
            self.assertRegex(header_text, rf"{var}:\s*\$\{{\{{\s*{expr}\s*\}}\}}")


if __name__ == "__main__":
    unittest.main()
