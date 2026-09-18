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
and every population still reads as expected. Each arm block's `Self::`
receivers are compared as a set against the arms read out of it as well, because
an arm rustfmt broke across lines still carries its receiver and would otherwise
be dropped in silence. Two arms naming one platform, or resolving one triple,
refuse for the same reason: a variant nothing can reach.

Every comparison here is between sets of fully-qualified names rather than their
sizes, because a size holds while its members are substituted underneath it: two
variants collapsing onto one platform, a variant absent from `ALL`, a binary
attached under a name no desktop derives. A set equality has the opposite blind
spot -- an extra member completes it instead of breaking it -- so each block is
also held against something a name this gate invented cannot satisfy. For the
arm blocks that is the receiver-against-arm comparison. For `ALL`, whose entries
carry no arms, it is the `[Self; N]` length rustc checks against those entries,
read out of the same match as the entries themselves so that no other occurrence
of that shape can supply it.

Non-code text is removed once, where the source is read, so nothing downstream
has raw text in scope to read by mistake. It is removed against each language's
own grammar rather than the spelling some defect happened to use: Rust's two
comment forms, nested to any depth, and neither of them opening inside a string
literal of either kind. The release side takes two
cuts, because that step's shell carries prose comments and its asset list is
data inside a quoted heredoc, where a `#` is neither a comment nor a path the
release attaches. The list is read out of that heredoc alone, and every line of
it must match an `artifacts/<dir>/<asset>` path whole -- a pattern that matched
only a line's tail read either kind of commented name as an attached one.

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
from typing import NamedTuple

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

#: The preview channel provisions its own servers, and `native_engine.rs` reaches
#: them by a different route than a release: `ResolvedArtifact` for a Preview key
#: looks the host's `target_triple()` up in the signed manifest's `binaries` map,
#: so a triple absent from that map is a desktop that resolves nothing. Nothing
#: above reads this workflow, and the two populations it declares are spelled four
#: separate times inside it -- the build matrix, the four artifact downloads, the
#: shell array that signs and uploads, and the jq object that writes the manifest.
#: Any one of them can be edited alone, which is exactly the drift this gate
#: exists to refuse; the release half is held to the same standard two files over.
#: The publish job's steps carry no `id`, so its signing step is named rather than
#: identified. Adding an `id` would be an edit to a publishing workflow made to
#: suit its own observer, and `.github/workflows/**` is a hard stop besides.
PREVIEW_WORKFLOW = ".github/workflows/preview-server.yml"
PREVIEW_BUILD_JOB = "build"
PREVIEW_PUBLISH_JOB = "publish"
PREVIEW_SIGN_STEP = "Sign binaries, publish manifest, and garbage-collect old pairs"
PREVIEW_ARTIFACT_PREFIX = "preview-server-"

#: `ALL` is the variant list `from_os_arch` iterates, and the two methods are the
#: arms it resolves them through. Each is anchored on its own name and bounded by
#: the closing brace at its indentation, so none of the three can be read as
#: another's contents -- the module also holds a free `target_triple()` function,
#: whose body carries no arms at all and so would read as an empty mapping rather
#: than as a wrong one.
#: `ALL`'s declared length is captured here rather than by a second pattern, so
#: the figure and the entries it is held against come out of one match. Read
#: separately it was satisfiable by any `[Self; N]` elsewhere in the file, which
#: is the whole forgery this cross-check exists to refuse.
MAPPING_ALL_BLOCK = re.compile(
    r"const ALL:\s*\[Self;\s*(\d+)\]\s*=\s*\[(.*?)\n    \];", re.S)
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
#: pattern spans the whole line, `artifacts/<dir>/` included, so the directory
#: half of each path -- which repeats the asset name -- is not counted a second
#: time and no line that merely *ends* in something asset-shaped can contribute a
#: name. End-anchoring alone read the tail of any line at all, which a set this
#: gate only ever grows cannot object to: a `#`-prefixed path names an asset the
#: release does not attach, and the desktop whose URL 404s stops being reported.
#: The capture takes the whole filename, `.exe` included, because that suffix is
#: part of the URL a desktop derives; the lazy name plus the trailing group is
#: what tells a binary line from its signature.
SIGN_LOOP = re.compile(r"for triple in((?:\s*\\\s*[\w.-]+)+)\s*;\s*do")
SIGN_TRIPLE = re.compile(r"[\w.-]+")
ASSET_LINE = re.compile(
    rf"^\s*artifacts/[\w.-]+/({SLIM_PREFIX}[\w.-]+?)(\.minisig)?$", re.M)
#: The attached list is the heredoc body, not the whole step, which is the other
#: half of the same cut: the pattern above decides what a line must look like,
#: and this decides where a line has to be to count at all. That step's shell
#: carries prose comments and conditional `echo`s around this list, and none of
#: it attaches an asset. Both halves are needed because adding a name is what the
#: subset check cannot object to -- `attached` is allowed to be a superset -- so a
#: name read from anywhere else reads as published and the desktop whose URL is
#: missing stops being reported. Nothing downstream can separate the two: a
#: commented path names the very asset whose absence was the finding, so the
#: population is narrowed at the read rather than compared afterwards.
ASSET_HEREDOC = re.compile(r"cat <<'EOF'\n(.*?)\n\s*EOF\b", re.S)

#: The preview publish step's two spellings, each narrowed to its own block before
#: any line is read, for the reason the heredoc above is: both sets are compared
#: by subset, so a name picked up from surrounding prose reads as provisioned and
#: silences the very desktop whose absence was the finding. The manifest keys are
#: taken from inside `binaries: {` alone -- the same step's jq also writes a
#: `data:` array of `{name, sha256, url}` objects, and a pattern matching any
#: quoted key followed by a brace would read those as platforms too.
#:
#: The array is not found by a pattern, because bash assigns it from spellings a
#: pattern does not name. Every `binaries` in the step must be one of three: the
#: standalone assignment (a `binaries=(` line, artifact lines, a `)` line), the
#: loops' `"${binaries[@]}"`, or the manifest's `binaries: {` line. Any other
#: refuses -- a one-line, appended, `declare`d or `mapfile`d array, a commented
#: one, prose. Names are matched as `_spliced` leaves them, because `declare`
#: assigns `"bin""aries=(...)"`; a line continuation onto a non-blank and
#: `$'...'` or `$"..."` quoting refuse instead, because bash joins a name across
#: the first and decodes one out of the second.
PREVIEW_ARRAY_MENTION = re.compile(r"(?<![A-Za-z0-9_])binaries(?![A-Za-z0-9_])")
#: What bash removes from a word before any command sees it: a quote, an escape,
#: and a line continuation -- which it deletes along with its newline rather than
#: leaving the break behind, so a word spelled across one is read as the word the
#: command is handed. That spelling is not exotic: the step ends every one of its
#: jq binding lines with one. Not a quoting model: it removes these wherever they
#: sit, which is sound only beside the two refusals below, the constructs that
#: would make removing them join or decode more than the step typed. The join
#: cannot fuse two words, because a continuation onto a non-blank refuses first
#: and what is left at the break is the next line's own indentation.
SHELL_SPLICE = re.compile(r"\\\n|[\\'\"]")
#: A line continuation bash joins onto a word, or that is not a continuation
#: at all (after `\\` or in a comment); either way the break is not one a
#: text reader can place.
PREVIEW_ARRAY_JOIN = re.compile(r"\\\n(?![ \t])")
#: `$'...'` and `$"..."` remove characters and decode escapes in a name.
PREVIEW_QUOTE_DECODE = re.compile(r"\$['\"]")
PREVIEW_ARRAY_OPEN = re.compile(r"[ \t]*binaries=\(")
PREVIEW_ARRAY_CLOSE = re.compile(r"[ \t]*\)")
PREVIEW_ARRAY_READ = '"${binaries[@]}"'
#: The only executable read this gate accepts. The array is the authority for
#: what is signed and uploaded, so a read that is not a loop over it is not
#: evidence that anything walks it: a later `echo "${binaries[@]}"` keeps a gate
#: that asks only for some read green while both loops iterate something else
#: entirely. Stated per read rather than as a count of reads, so a third loop, a
#: merged single loop, or a refactor to another iterator each land on the rule.
PREVIEW_ARRAY_LOOP = re.compile(
    r'[ \t]*for ([A-Za-z_]\w*) in "\$\{binaries\[@\]\}"; do')
#: What a loop over the array must run on the loop's own variable. Naming the
#: commands is what ties the array to what bash does with it: a loop that walks
#: `binaries` while the signing and upload commands walk another collection
#: leaves the validated set unpublished, and a gate that asked only for a loop
#: stays green over it. Looked for across every loop and unioned, so one merged
#: loop running all three and separate loops running one each are equally legal.
#:
#: Where a command sits, not what it reaches: a consumer under a condition that
#: is never true, or in a function nothing calls, is counted like any other, and
#: a call is read as the word it spells rather than as the body of the function
#: behind it, so a helper that rebinds the loop variable rebinds nothing any line
#: here spells. Decoys built either way are admitted. Reading a call site as its
#: callee, like modelling reachability, is a different reader than this one, and
#: its absence is the older admission -- `_preview_artifacts` reads the array
#: assignment the same way.
#:
#: The text between the command word and the option is crossed as words rather
#: than as characters. `.*?` in its place reaches inside a quoted argument, where
#: the option it finds is a literal bash passes along rather than one the command
#: reads: `--cache-control '--file "$binary" x'` is one word, and matching its
#: interior counts an upload of a file the step never uploads. The option must
#: begin the word it sits in for the same reason, and the subcommand must be the
#: whole word it sits in: bash hands `--cache-control=--file` and `put.x` over as
#: single words, and wrangler is passed no `--file` by the first and no `put` by
#: the second. A boundary spelled as characters that must not precede a token
#: admits every character the list forgot, so these patterns require the
#: separator that must be there instead.
#:
#: The gap stays inside one command as well as outside one word. An unquoted
#: `;`, `&`, `|` or newline ends the command the subcommand opened, so an option
#: past one is a different command's: `put ... --file "$other" ; echo --file
#: "$binary"` hands wrangler no `--file` for the binary, which then goes
#: unuploaded while the array is walked. Quoted and escaped separators are words
#: bash passes along, so they stay ordinary text -- the step's own
#: `--cache-control "public, max-age=31536000, immutable"` is crossed like any
#: other argument.
SHELL_WORD_GAP = r"""(?:\\.|[^'"\\;&|\n]|'[^']*'|"(?:\\.|[^"\\])*")*?"""
SHELL_FILE_OPTION = r"[ \t]--file[ \t]+"
PREVIEW_CONSUMERS = (
    ("signs it", r'^[ \t]*sign[ \t]+{binary}(?=[ \t;&|]|$)'),
    ("uploads it", r'^[ \t]*(?:npx[ \t]+)?wrangler r2 object put(?=[ \t])'
                   + SHELL_WORD_GAP + SHELL_FILE_OPTION
                   + r'{binary}(?=[ \t;&|]|$)'),
    ("uploads its signature",
     r'^[ \t]*(?:npx[ \t]+)?wrangler r2 object put(?=[ \t])'
     + SHELL_WORD_GAP + SHELL_FILE_OPTION
     + r'{signature}(?=[ \t;&|]|$)'),
)
#: `do` closing a loop header, and `done` opening a line. Counted so a loop
#: nested in the body does not end it early. The opener must *begin* with the
#: keyword whose header the `do` closes: a line merely ending in that word --
#: `echo nothing to do` -- opens nothing, and reading one as an opener pushes the
#: body past its own `done` and counts commands that sit outside the loop, which
#: is the fail-open direction. `done` keeps its line to itself but for a
#: redirection or a list operator, neither of which changes which loop it closes.
SHELL_LOOP_KEYWORD = r"[ \t]*(?:for|while|until|select)(?![\w-])"
PREVIEW_LOOP_OPEN = re.compile(SHELL_LOOP_KEYWORD
                               + r".*(?:^|[ \t;])do$")
PREVIEW_LOOP_CLOSE = re.compile(r"[ \t]*done(?![\w-])")
#: A header that puts its `do` on a line of its own, which POSIX allows and which
#: opens exactly what the joined spelling above does. Read as two lines rather
#: than refused, so a loop written that way is delimited instead of closing its
#: parent early -- which truncates the body and reports a member as missing, a
#: refusal naming a reason that is not the reason. The keyword line is what
#: licenses the bare `do`: a `do` no header precedes opens nothing, because
#: opening on it would carry the body past the `done` that closes the real loop.
PREVIEW_LOOP_KEYWORD = re.compile(SHELL_LOOP_KEYWORD)
PREVIEW_LOOP_DO = re.compile(r"[ \t]*do(?![\w-])")
#: A line that gives the loop variable a value the loop did not. Past one the
#: variable no longer names an element of the array, so `sign "$binary"` below it
#: signs whatever was rebound -- the loop still walks `binaries` and publishes
#: none of it. The upload prefix is held against this too, where what a line
#: below carries is the path every object goes to rather than the file signed.
#:
#: Stated as the complement of a use, the name written any way but as an
#: expansion of itself, rather than as the commands that bind it. A list of those
#: is falsified by the spelling it omits, and every omission here is silent:
#: `printf -v binary`, `let "binary = 1"` and `(( binary = 1 ))` each sign and
#: publish the wrong file under the step's own `set -euo pipefail`, so a reader
#: enumerating binding commands fails open on the one it has not met yet.
#:
#: The complement is taken over the lines bash hands this step as its own words,
#: which is narrower than every spelling that binds the name: an assignment
#: inside `$(( ... ))` is blanked before this is read, and `${binary:=x}` is one
#: of the expansions counted as a use. Neither publishes a wrong file quietly --
#: arithmetic binds a number, which the step's own `sign` then fails on under
#: `set -euo pipefail`, and the default never fires, every element of the array
#: having been proven a non-empty path already.
#:
#: The expansion spellings are matched first so each consumes its own copy of the
#: name and only a bare occurrence is left to report. Matched against the whole
#: line and applied ahead of the nesting count, because the rebinding spelling
#: that hides best is a nested loop header, which ends in `do` like any other.
#:
#: Read as `_spliced` leaves the line, because every command that binds the name
#: takes it as an argument, where bash removes quotes first: `printf -v b\inary`
#: and `printf -v "binary"` each bind what the loop bound. An assignment prefix is
#: the one place that quoting keeps the name from being a name -- bash reads
#: `b\inary=x` as a command -- and it is reported as the bare occurrence it is
#: rather than modelled, because the step's own `set -e` stops at that command
#: and the refusal quotes the line it read.
PREVIEW_REBIND = (r'\$\{{[#!]?{name}[^}}]*\}}'
                  r'|\${name}(?!\w)'
                  r'|(?<!\w)({name})(?!\w)')
PREVIEW_MANIFEST_OPEN = re.compile(r"[ \t]*binaries: \{")
PREVIEW_ARRAY_LINE = re.compile(
    r"[ \t]*artifacts/[\w.-]+/phase-server-([\w.-]+?)(\.exe)?")
#: Commands that run text the step does not hold as lines of its own; each
#: takes an argument, so a word followed by a blank is the command.
PREVIEW_EVAL = re.compile(r"(?<![^\s;&|(<>$])(?:eval|source|\.)[ \t]")
#: Words that make bash rewrite a line before parsing it: an alias replaces a
#: command word with its text, and history expansion replaces `!` designators.
PREVIEW_REWRITE = re.compile(r"(?<!\w)(?:alias|BASH_ALIASES|history)(?!\w)")
#: A heredoc operator with a bare or wholly quoted delimiter. The scanner below
#: models no other delimiter spelling.
SHELL_HEREDOC = re.compile(
    r"<<(-?)[ \t]*(?:([A-Za-z_]\w*)|'([A-Za-z_]\w*)'|\"([A-Za-z_]\w*)\")"
    r"(?=[\s;&|()<>]|$)")
SHELL_CASE = re.compile(r"case(?=[\s;&|()<>])")
SHELL_METACHARS = " \t\n;&|()<>"
SHELL_CONTEXTS = {
    "single": "a single-quoted string",
    "double": "a double-quoted string",
    "comsub": "a `$( ... )` command substitution",
    "backtick": "a backtick command substitution",
    "param": "a `${ ... }` expansion",
}
#: The frames whose text the step never hands to a command as its own words: a
#: comment is not run at all, and each substitution form runs as its own command
#: whose output becomes a word here. Heredoc bodies are blanked beside them,
#: though a heredoc is not a frame.
#:
#: `single` and `double` are deliberately not members, and cannot become ones: a
#: quoted word is part of the command bash runs, so blanking `single` takes the
#: manifest object's match count to zero and blanking `double` the upload
#: prefix's. `param` carries no authority any reader takes out of `executed`
#: today -- the array's reads are scanned from the step as written -- and it
#: stays out for the same reason, so that an authority which moves inside a
#: `${ ... }` is read rather than erased.
SHELL_HIDDEN = {"comment", "comsub", "backtick"}
#: The command that writes the manifest, and the region both of the manifest's
#: authorities are read out of -- each from its own part of it. `jq -n` is that
#: command's whole identity: the step's other jq invocations all read a file or a
#: here-string, and this one alone builds the object the desktop resolves against
#: out of nothing. Exactly one, because two of them make which object is published
#: a question this gate cannot answer from the text -- and answering it wrongly is
#: what lets a second `jq -n` supply a map the real one no longer writes.
#:
#: Both of jq's spellings of that option, so the one the step does not use today
#: is read rather than counted as a `jq -n` this gate never found. One command
#: still contributes one start: the spelling is anchored on the command word, and
#: a command carries one of those.
PREVIEW_MANIFEST_JQ = re.compile(r"(?<![\w-])jq[ \t]+(?:-n|--null-input)(?![\w-])")
#: One word of a command as bash splits it, or one operator that is not a word
#: at all. Quoted strings and escapes are crossed whole; whitespace ends a word.
#: A line continuation is a separator here rather than a joiner, which is sound
#: only because a continuation onto a non-blank has already refused -- what is
#: left at every break is the next line's own indentation.
#:
#: `<`, `>` and the parentheses are matched apart, because bash hands none of
#: them to the command and the word after a redirection is a file name rather
#: than an argument. Left in the word class they would vanish from the walk and
#: their target would be counted as one of the command's own words.
SHELL_WORD = re.compile(
    r"""(?:\\[^\n]|[^'"\\\s;&|<>()]|'[^']*'|"(?:\\.|[^"\\])*")+|[<>()]""")
SHELL_OPERATORS = frozenset("<>()")
#: jq's own option table (`jq --help`, 1.7.1), split by how many words each
#: option takes as its values -- plus `--argfile`, which jq 1.7 removed and older
#: jq still honours. `--args` and `--jsonargs` take none: jq's usage puts the
#: filter ahead of the positional values they consume.
#:
#: A word beginning with `-` that is in neither refuses, because either guess
#: about it moves which word the program is: reading it as a flag makes jq's
#: filter whatever that option's value spells, and reading it as a value-taker
#: skips the filter itself. A jq release adding an option is what retires this.
JQ_FLAGS = frozenset(
    """-n --null-input -R --raw-input -s --slurp -c --compact-output
    -r --raw-output --raw-output0 -j --join-output -a --ascii-output
    -S --sort-keys -C --color-output -M --monochrome-output --tab --unbuffered
    --stream --stream-errors --seq --args --jsonargs -e --exit-status
    -V --version --build-configuration -h --help""".split())
JQ_VALUE_OPTIONS = {"--indent": 1, "-f": 1, "--from-file": 1, "-L": 1,
                    "--arg": 2, "--argjson": 2, "--slurpfile": 2,
                    "--rawfile": 2, "--argfile": 2}
#: How far that command runs: words, up to the first separator bash does not read
#: as part of one. A quoted string is crossed whole, newlines included, because
#: the jq program is one such word and the bindings ahead of it are continued
#: across lines; an unquoted `;`, `&`, `|` or newline is where the next command
#: begins and the manifest command stops owning what follows.
PREVIEW_COMMAND_TEXT = re.compile(
    r"""(?:\\.|[^'"\\;&|\n]|'[^']*'|"(?:\\.|[^"\\])*")*""", re.S)
#: Read out of the manifest command's own text, and matched exactly once in the
#: step: taking the first match let a decoy object -- one a heredoc body carries,
#: or one spelled ahead of the real jq program -- substitute the whole map, and
#: reading it from anywhere in the step let a decoy elsewhere supply it after the
#: real map was renamed away. The object itself lives inside the jq program's
#: single-quoted word, which is a word bash hands to jq, so it is held to being
#: executable text and not to beginning a command line; requiring the latter would
#: refuse the step this gate exists to read.
#: Anchored on the `data:` key that follows it, because every entry *inside* the
#: object also ends in `},` -- stopping at the first one would read a single
#: platform's interior as the whole map and stranding the other three would look
#: like a finding rather than like a pattern that stopped early.
PREVIEW_MANIFEST_BLOCK = re.compile(
    r"binaries:\s*\{\n(.*?)\n\s*\},\s*\n\s*data:", re.S)
PREVIEW_MANIFEST_KEY = re.compile(r'^\s*"([\w.-]+)":\s*\{\s*$', re.M)
#: A signature beside every binary: the desktop derives `sig_url` as well as
#: `url`, so a key carrying only one of them resolves an artifact it cannot
#: verify. Matched per key rather than counted, so the report names which.
#: The whole right-hand side, not its first literal: each URL is a concatenation
#: (`"...preview-server/" + $fingerprint + "/phase-server-<triple>"`), so the
#: binary's name is in the second string. A pattern stopping at the first would
#: never see the name it exists to hold the key against.
PREVIEW_MANIFEST_URL = re.compile(r"^\s*(url|sig_url):\s*(.+)$", re.M)
#: The upload path, read out of the step's executable text and out of a command
#: line, because an assignment bash never runs sets nothing. Every object goes to
#: `$prefix/$name`, so this assignment -- not a spelling of it kept here -- is
#: what fixes the path a manifest entry has to name. A gate holding its own copy
#: of the path passes whenever the manifest agrees with that copy, including when
#: both disagree with the upload the step performs.
#:
#: This spelling is what the value is read out of, and every other line naming
#: the name is held against `PREVIEW_REBIND` instead, the way the loop variable
#: is: `prefix=desktop/x/$F` and `export prefix="desktop/x/$F"` are assignments
#: this pattern does not match, and bash keeps the last value assigned, so a
#: reader that merely failed to match them checks the manifest against a path
#: nothing is uploaded to. The value stays on the quoted spelling, where the `"`
#: after the variable is what ends its name: quote removal would leave
#: `"$FING"PRINT` reading as a variable bash never expands.
PREVIEW_PREFIX_ASSIGN = re.compile(r'^\s*prefix="([^"\n]*)"\s*$', re.M)
#: jq's own binding of a shell variable to the name its program uses, read out of
#: the manifest command's own words. The prefix is shell (`$FINGERPRINT`) and the
#: manifest URL is jq (`$fingerprint`); this flag is the only thing that says the
#: two are one value, so the pairing is read from it rather than inferred from
#: the two spellings looking alike. It binds nothing outside the command that
#: carries it, so a pairing read from the rest of the step says the manifest names
#: the uploaded path while the program expands whatever this command did bind.
#: The bindings sit on backslash-continued lines of the jq command, so like the
#: manifest object they are held to being executable text rather than to
#: beginning a command line.
#: Held to the quoted spelling, unlike the option and name counted below: the `"`
#: after the variable is what ends its name, and quote removal leaves
#: `"$FING"PRINT` reading as a variable bash never expands.
PREVIEW_JQ_ARG = re.compile(r'--arg\s+(\w+)\s+"\$(\w+)"')
#: Every jq option that binds a name into the program's namespace, from jq's own
#: option table (`jq --help`): `--arg`, `--argjson`, `--slurpfile`, `--rawfile`,
#: and `--argfile`, which jq 1.7 removed and older jq still honours. `--args` and
#: `--jsonargs` bind `$ARGS.positional`, which names nothing, and are not
#: members. This is what counts the selected name's bindings; the pairing of a jq
#: name to a shell variable is still read from `--arg name "$VAR"` alone, because
#: no other spelling states that the two are one value. A family rebinding the
#: name leaves the manifest URLs checked against a value jq may not be expanding.
#: Read against `_spliced` text, so both words of a binding are counted as jq
#: receives them rather than as the step spells them: a quoted or escaped option
#: hides a collision exactly as well as a quoted or escaped name does.
#: The name ends on any character a word cannot continue through, so a binding at
#: the end of a line or of a continuation is counted too; `-` is excluded so a
#: hyphenated word is not read as the bare name plus a remainder.
PREVIEW_JQ_BIND = re.compile(
    r"--(argjson|arg|slurpfile|rawfile|argfile)[ \t]+(\w+)(?=[^\w-]|$)")
#: The one comparand this gate cannot derive: the step uploads into the R2 bucket
#: `phase-rs-data`, and the bucket's public hostname is Cloudflare configuration
#: that this repository does not contain. Everything after it -- prefix,
#: fingerprint variable, file name -- is read from the step itself.
PREVIEW_DATA_HOST = "https://data.phase-rs.dev/"
#: A jq concatenation split into the pieces whose identity matters. String
#: contents are captured whole, so whitespace *inside* a literal stays
#: significant -- R2 holds nothing under `preview- server/` -- while whitespace
#: *between* tokens is dropped, so reformatting the expression stays free. A
#: character matching nothing else becomes an `other` token, so a reshaped
#: expression cannot quietly tokenise into the expected sequence.
PREVIEW_JQ_TOKEN = re.compile(r'"((?:[^"\\]|\\.)*)"|\$(\w+)|([()+])|(\S)')


def _jq_tokens(expression: str) -> list[tuple[str, str]]:
    """A jq expression as typed tokens, less one optional trailing comma.

    The manifest writes `url: (...),` ahead of `sig_url:`, so the first of the
    pair carries a separator the second does not.
    """
    tokens: list[tuple[str, str]] = []
    for literal, variable, operator, other in PREVIEW_JQ_TOKEN.findall(
            expression.strip().removesuffix(",")):
        if variable:
            tokens.append(("var", variable))
        elif operator:
            tokens.append(("op", operator))
        elif other:
            tokens.append(("other", other))
        else:
            tokens.append(("str", literal))
    return tokens

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
    return _strip_rust_comments(path.read_text(encoding="utf-8"))


#: A raw string's opener, matched whole rather than found by looking at what
#: precedes an `r`. `br#"` and `cr#"` are raw strings whose `r` is preceded by an
#: identifier character, so a guard reading that character alone declines to
#: recognise them: it is the prefixed form of the very token it is guarding. The
#: file this gate reads carries six `br#"` literals today.
RAW_OPEN = re.compile(r'[bc]?r(#*)"')
#: A char literal, against the Reference's production rather than against the
#: forms this scanner happened to remember: an ordinary character, an escape, a
#: `\xNN` byte escape, or a `\u{...}` unicode escape. The two multi-character
#: escapes are why the bare `\\.` spelling was wrong -- it consumes exactly two
#: characters and then demands the closing quote, so `'\u{41}'` matched nothing,
#: its trailing quote was left loose, and that quote paired with the next one two
#: characters along and swallowed a real `"`.
#:
#: The unicode escape's bound is on hex digits, not on characters between the
#: braces: `'\u{1_F_6_0_0}'` compiles, and counting its underscores against the
#: budget refused source rustc accepts. Overlong stays refused -- seven digits is
#: not a char literal, and this gate does not read what the compiler rejects.
CHAR_LIT = re.compile(
    r"b?'(?:\\u\{_*(?:[0-9a-fA-F]_*){1,6}\}|\\x[0-9a-fA-F]{2}|\\.|[^'\\\n])'")
#: A lifetime or loop label: the only other token that opens with a quote and the
#: reason an unrecognised quote cannot simply be assumed to be a literal. It has
#: no closing quote, so it is consumed as itself.
LIFETIME = re.compile(r"'[A-Za-z_][A-Za-z0-9_]*")


def _raw_close(text: str, opener: re.Match[str]) -> int:
    """End of the raw string whose opener this match covers.

    Inside one, `\\` escapes nothing and `//` is data, so a scanner reading it as
    an ordinary literal leaves literal state open at the first `"` the body
    carries and applies every comment rule after that point to code. An
    unterminated opener runs to end of input, which preserves the remainder
    rather than dropping it; it does not compile, so no tree reaches it.
    """
    hashes = opener.group(1)
    end = text.find('"' + hashes, opener.end())
    return len(text) if end < 0 else end + 1 + len(hashes)


def _strip_rust_comments(text: str) -> str:
    """`text` with every Rust comment removed, before any pattern reads it.

    Against the comment grammar whole rather than one of its spellings: `//`
    runs to end of line, `/* */` nests to any depth, and neither opens a comment
    inside a string literal, so `"http://x"` keeps its arm. Newlines survive, so
    the one-arm-per-line shape the callers read is unchanged.

    Nesting is why this scans rather than substitutes: `/\\*.*?\\*/` closes the
    outer comment at the inner `*/` and leaves the remainder of it standing as
    code. A line-oriented rule cannot serve either, which is where this parts
    company with `source_census::code` -- a block comment spans lines, and
    whether the quotes left of a `//` on one line are balanced is a different
    question from whether that `//` stands inside a literal. Literal state is
    tracked instead, so this needs no claim about which blocks carry literals;
    `ALL`'s entries carry none.

    A name written inside a string literal is not a comment and is still read.
    It refuses rather than passing -- as a receiver written twice in an arm
    block, or against `ALL`'s declared length -- because the patterns below are
    not literal-aware either: an escaped quote ends a captured value early, so a
    corrupted triple reports on the published-asset axis instead of refusing.

    Every opener in Rust's literal grammar, because this reads whole source
    files and an enumeration of them is falsified by the member it omits. Each
    omission has the same consequence: a quote that opens no literal is read as
    one, literal state is left open, and every comment rule after that point is
    applied to code. The members are the plain, byte and C-string quotes, the
    raw forms with any hash count, and the char literals -- and each is matched
    whole, at its own start, rather than recognised by the character before it,
    which is a test the prefixed forms fail by construction.

    That enumeration has been wrong five times, so it is no longer trusted to be
    complete: a quote matching no opener refuses rather than being read as code.
    The bound is the point. A member omitted from here now costs a named refusal
    a contributor can act on, instead of a file silently mis-stripped -- which is
    the only failure this gate cannot detect in itself.
    """
    out: list[str] = []
    i, n, depth = 0, len(text), 0
    while i < n:
        pair = text[i:i + 2]
        if depth:
            if pair == "/*":
                depth += 1
                i += 2
            elif pair == "*/":
                depth -= 1
                i += 2
            else:
                if text[i] == "\n":
                    out.append("\n")
                i += 1
        elif pair == "/*":
            depth += 1
            i += 2
        elif pair == "//":
            end = text.find("\n", i)
            if end < 0:
                break
            i = end
        elif ((i == 0 or not (text[i - 1].isalnum() or text[i - 1] == "_"))
              and (opener := RAW_OPEN.match(text, i)) is not None):
            close = _raw_close(text, opener)
            out.append(text[i:close])
            i = close
        elif (lit := CHAR_LIT.match(text, i)) is not None:
            out.append(lit.group())
            i = lit.end()
        elif (life := LIFETIME.match(text, i)) is not None:
            out.append(life.group())
            i = life.end()
        elif text[i] == "'":
            raise Refusal(
                f"{MAPPING_SOURCE} carries a quote at offset {i} that opens no "
                f"token this gate knows ({text[i:i + 12]!r}): it is neither a "
                "char literal nor a lifetime. Reading it as code would leave "
                "literal state open and apply every comment rule after it to "
                "code, so this refuses instead of guessing")
        elif text[i] == '"':
            out.append('"')
            i += 1
            while i < n:
                if text[i] == "\\":
                    out.append(text[i:i + 2])
                    i += 2
                    continue
                out.append(text[i])
                i += 1
                if text[i - 1] == '"':
                    break
        else:
            out.append(text[i])
            i += 1
    return "".join(out)


def _mapping_block(text: str, pattern: re.Pattern[str], what: str) -> str:
    """One block's code, out of a source every comment was already removed from.

    The single place both arm blocks are read through. Comments are gone before
    this runs, at the read: stripping per block instead left the `//` that opens
    one outside the captured group, so a comment carrying a declaration's shape
    moved the block boundary and its contents were read as code.
    """
    match = pattern.search(text)
    if match is None:
        raise Refusal(f"{MAPPING_SOURCE} has no readable `ServerPlatform::{what}`; "
                      "it was renamed or reformatted, and the mapping cannot be "
                      "read")
    return match.group(1)


def _all_entries(text: str) -> tuple[int, list[str]]:
    """`ALL`'s declared length and its entry names, out of one match.

    Two matches let the length come from somewhere the entries did not, and any
    `[Self; N]` in the file would then serve: that is how this cross-check was
    satisfiable by a sentence of prose.
    """
    match = MAPPING_ALL_BLOCK.search(text)
    if match is None:
        raise Refusal(f"{MAPPING_SOURCE} has no readable `ServerPlatform::ALL`; "
                      "it was renamed or reformatted, and the mapping cannot be "
                      "read")
    return int(match.group(1)), MAPPING_RECEIVER.findall(match.group(2))


def _refuse_phantom_all_entry(declared: int, listed: list[str]) -> None:
    """Refuse when `ALL` reads as more or fewer entries than rustc counts.

    `ALL` is compared to the two arm blocks by name, and a name this gate
    invented is what that comparison cannot object to: a phantom entry makes
    `set(listed)` complete rather than short, so the comparison passes and the
    variant genuinely missing from `ALL` goes unreported -- the way this block
    failed open, where the arm blocks' receiver-against-arm comparison already
    refused. `[Self; N]` is checked by rustc against the entries themselves, so
    it holds against a name no entry produced.
    """
    if len(listed) != declared:
        raise Refusal(
            f"{MAPPING_SOURCE}: ServerPlatform::ALL is declared `[Self; "
            f"{declared}]` but this gate read {len(listed)} entry name(s) in it: "
            f"{listed}. rustc counts the entries itself, so a name here it does "
            "not count came out of something that is not an entry -- text no "
            "comment removal took out, or a name inside a string literal. More "
            "names than rustc counts is the direction that would otherwise pass: "
            "an extra one completes the by-name comparison instead of breaking "
            "it, hiding a variant absent from ALL")


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
    declared, listed = _all_entries(text)
    _refuse_phantom_all_entry(declared, listed)
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
    # Both axes, because neither collision implies the other. Two variants can
    # share a bare triple while deriving different names -- exactly one of them
    # `windows` appends `.exe` to only its own -- and two distinct triples can
    # collapse onto one derived name when they differ only by that suffix. A
    # check keyed on either alone reads the other's collision as clean.
    # The derived name first, because it is the narrower report: when both axes
    # collide, the single URL is what a desktop actually requests.
    _refuse_shared({name: slim_asset(os_name, triple)
                    for name, (os_name, _, triple) in mapped.items()},
                   "target_triple's derived asset name",
                   "derive one asset name, so both desktops request a single URL "
                   "and the name the second should have asked for is never "
                   "checked against the release at all")
    _refuse_shared({name: triple
                    for name, (_, _, triple) in mapped.items()},
                   "target_triple",
                   "resolve the same engine triple, so both desktops are served "
                   "one binary built for one of their platforms and the other "
                   "runs an engine compiled for a machine it is not. Differing "
                   "derived names do not separate them: when exactly one of the "
                   "variants is `windows`, its own `.exe` makes the two names "
                   "differ and the asset-name check above sees no collision")
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

    jobs = (workflow or {}).get("jobs") or {}
    job = jobs.get(BUILD_JOB)
    if not isinstance(job, dict):
        raise Refusal(f"{SHELL_RELEASE}: job '{BUILD_JOB}' is absent; the job "
                      "that publishes desktop packages was renamed, and this "
                      "gate no longer knows which platforms ship")

    # The published set is anchored on one job id, so a second job publishing
    # desktops is a population this gate never walks. The axis refusal below is
    # scoped to this job alone and does not reach siblings, so a sibling's matrix
    # is read here in every shape that can produce a platform: a job runs the
    # product of its axes with each `include` entry merged in, so `os` on an axis
    # and `arch` on an entry ship a platform neither spells by itself. Reading
    # only one shape reads a subset of what publishes.
    def _publishes_desktops(name: str, other: object) -> bool:
        strategy = other.get("strategy") if isinstance(other, dict) else None
        if not isinstance(strategy, dict) or "matrix" not in strategy:
            return False
        matrix = strategy["matrix"]
        if not isinstance(matrix, dict):
            raise Refusal(f"{SHELL_RELEASE}: job '{name}' declares a "
                          f"strategy.matrix this gate cannot read ({matrix!r}), "
                          "so whether it publishes desktops is unknown. A job "
                          "that might is not one to pass over")
        axes = set(matrix) - {"include", "exclude"}
        # The same unreadability one level down, and the level a filter would
        # swallow: a string iterates characters and a mapping iterates keys, so
        # `[e for e in include if isinstance(e, dict)]` turns either into an empty
        # list, and an empty list publishes nothing. That is the empty set passing
        # a subset check, which is what this gate refuses everywhere else --
        # including on this same key when it belongs to the job being read.
        include = matrix.get("include")
        if include is not None and not (isinstance(include, list)
                                        and all(isinstance(e, dict) for e in include)):
            raise Refusal(f"{SHELL_RELEASE}: job '{name}' declares a "
                          f"strategy.matrix.include this gate cannot read "
                          f"({include!r}), so whether it publishes desktops is "
                          "unknown. A job that might is not one to pass over")
        entries = include or []
        return {"os", "arch"} <= axes or any(
            {"os", "arch"} <= (axes | e.keys()) for e in entries)

    others = sorted(name for name, other in jobs.items()
                    if name != BUILD_JOB and _publishes_desktops(name, other))
    if others:
        raise Refusal(f"{SHELL_RELEASE}: {others} also declare matrix entries "
                      f"carrying both `os` and `arch`, so they publish desktops "
                      f"too, but this gate reads only '{BUILD_JOB}'. Every "
                      "platform they ship is one no ServerPlatform variant was "
                      "checked against")

    matrix = (job.get("strategy") or {}).get("matrix", {})
    # A matrix runs its product axes as well as its `include` entries, so reading
    # `include` alone reads a subset of what publishes and the platforms an axis
    # contributes strand silently. This gate identifies a platform by an (os,
    # arch) pair on one entry, which a product has no single entry for, so an
    # axis is a shape it cannot read rather than one it reads partially.
    if isinstance(matrix, dict) and (axes := sorted(set(matrix)
                                                    - {"include", "exclude"})):
        raise Refusal(f"{SHELL_RELEASE}: {BUILD_JOB} declares matrix axes "
                      f"{axes} beside `include`; every combination of those runs "
                      "and publishes a desktop too, so the published platform "
                      "set is larger than the `include` list this gate reads")
    include = matrix.get("include") if isinstance(matrix, dict) else None
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


def _workflow_job(workflow: str, job_id: str, reads: str) -> dict[str, object]:
    """One job of one workflow: the only place a workflow file is opened."""
    path = ROOT / workflow
    if not path.is_file():
        raise Refusal(f"{workflow} does not exist; {reads} cannot be read")
    try:
        parsed = yaml.safe_load(path.read_text(encoding="utf-8"))
    except yaml.YAMLError as exc:
        raise Refusal(f"{workflow} is not parseable YAML: {exc}") from exc

    job = ((parsed or {}).get("jobs") or {}).get(job_id)
    if not isinstance(job, dict):
        raise Refusal(f"{workflow}: job '{job_id}' is absent or unreadable; the "
                      f"job that {reads} depends on was renamed or reshaped, and "
                      "a job this gate cannot find declares nothing rather than "
                      "declaring an empty set")
    return job


def _step_bodies(workflow: str, job_id: str, reads: str) -> dict[str, str]:
    """Each step's shell body, reachable by `id` and by `name`.

    Both, because the two workflows this gate reads identify their steps
    differently: the release job gives its steps ids, while the preview publish
    job names them and gives ids to neither. A duplicate key refuses rather than
    resolving to one of the two bodies -- picking either would read one step's
    shell as another's, and the set that came back would be a real set read off
    the wrong step, which no downstream subset check can tell from the right one.
    """
    steps = _workflow_job(workflow, job_id, reads).get("steps")
    if not isinstance(steps, list):
        raise Refusal(f"{workflow}: job '{job_id}' declares no steps list; the "
                      f"job that {reads} depends on was reshaped, and this gate "
                      "no longer knows which triples ship")
    bodies: dict[str, str] = {}
    for step in steps:
        if not isinstance(step, dict):
            continue
        run = str(step.get("run") or "")
        for key in (step.get("id"), step.get("name")):
            if not isinstance(key, str):
                continue
            if key in bodies and bodies[key] != run:
                raise Refusal(f"{workflow}: job '{job_id}' has two steps "
                              f"answering to '{key}', so {reads} would be read "
                              "off whichever this gate happened to keep")
            bodies[key] = run
    return bodies


def _step_body(bodies: dict[str, str], step: str, workflow: str, job_id: str,
               reads: str) -> str:
    body = bodies.get(step)
    if body is None:
        raise Refusal(f"{workflow}: {job_id} has no step '{step}', so {reads} "
                      "cannot be read")
    return body


def published_assets() -> set[str]:
    """Every `phase-server-slim-*` name the release attaches, signatures included.

    A desktop derives two URLs from its platform, the binary and the signature
    beside it, so each is its own fully-qualified identity here rather than one
    identity carrying a flag. Whichever of them a release fails to attach is then
    the name that comes back missing, instead of a pair that drops out of a count.
    """
    bodies = _step_bodies(RELEASE_WORKFLOW, RELEASE_JOB,
                          "the set of published slim server assets")

    loop = SIGN_LOOP.search(
        _step_body(bodies, SIGN_STEP, RELEASE_WORKFLOW, RELEASE_JOB,
                   "the triples the release signs"))
    if loop is None:
        raise Refusal(f"{RELEASE_WORKFLOW}: {SIGN_STEP} has no readable `for "
                      "triple in ...` loop; the signed set was reshaped, and a "
                      "set this gate cannot read is not an empty one")
    signed = set(SIGN_TRIPLE.findall(loop.group(1)))
    listing = ASSET_HEREDOC.search(
        _step_body(bodies, ASSET_STEP, RELEASE_WORKFLOW, RELEASE_JOB,
                   "the assets the release attaches"))
    if listing is None:
        raise Refusal(f"{RELEASE_WORKFLOW}: {ASSET_STEP} has no readable `cat "
                      "<<'EOF'` asset list; the attached set was reshaped, and a "
                      "set this gate cannot read is not an empty one")
    lines = ASSET_LINE.findall(listing.group(1))
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


class ShellStep(NamedTuple):
    """The signing step read as shell: what bash runs, and where commands start.

    `executed` is `raw` with every region the step does not hand to a command as
    its own words blanked. That is narrower than "text bash does not run": bash
    does run `$( ... )` and backticks, but as a separate command whose output
    becomes a word here, so an authority moved inside one is not this step saying
    anything. Such an authority is then not read at all and its reader refuses
    with a found count of zero, which is the fail-closed direction; the real step
    carries none.

    `line_context` answers what each line start sits inside, so the four
    authorities read out of one step each ask about their own line without the
    body being walked again.
    """

    raw: str
    executed: str
    line_context: dict[int, str | None]
    where: str

    def context(self, at: int) -> str | None:
        """What the line holding `at` sits inside, or None when bash runs it."""
        return self.line_context[self.raw.rfind("\n", 0, at) + 1]


def _shell_step(body: str, where: str) -> ShellStep:
    """One walk over the whole step: what bash runs, and where commands start.

    Bash runs a line as a command only when it is not inside a quoted string, a
    heredoc body, a command substitution, or an unclosed `(` or `[` --
    arithmetic, a pattern, a subscript, a subshell. Constructs outside the
    modelled set refuse, because one misread construct moves every quote after
    it. `$((` is scanned as a substitution holding parentheses, and a `)` with no
    `(` open is a `case` pattern's and closes nothing. Branches, functions and
    pipelines are not modelled: the line is a command inside them too.

    Every line start is recorded, the ones a heredoc swallows included, and the
    walk runs to the end of the body rather than to one authority's offset: four
    authorities are read out of this step and the last of them sits after every
    construct the first passes through.

    Blanking leaves `executed` the same length as `body` with its newlines in
    place, so offsets, line numbers and `re.M` anchoring are the same in both.
    The blank is NUL rather than a space, so a blanked line satisfies no
    `^\\s*...$` authority pattern and splices none of its neighbours together.
    """
    frames: list[list] = [["code", 0, 0]]
    heredocs: list[tuple[str, bool, bool, int]] = []
    word_start, joined, i = True, False, 0
    line_context: dict[int, str | None] = {}
    out = list(body)

    def unmodelled(what: str) -> Refusal:
        line = body.count("\n", 0, i) + 1
        return Refusal(f"{where} uses {what} on line {line}, which this gate does "
                       "not read as shell; a construct read wrongly moves every "
                       "quote after it, so which of the step's lines bash runs "
                       "cannot be told")

    def context_now() -> str | None:
        if len(frames) > 1:
            return SHELL_CONTEXTS[frames[-1][0]]
        if frames[0][1] or frames[0][2]:
            return ("an unclosed `(` or `[`, which bash reads as one word or "
                    "expression")
        if joined:
            return "the line before it, which ends in a line continuation"
        return None

    def blank(start: int, end: int) -> None:
        # Clamped here rather than at the callers: an operator's own length is
        # what two of them pass, and a body ending mid-operator is a step whose
        # last construct is unterminated, which the walk already reads as running
        # to the end of the body.
        for at in range(start, min(end, len(body))):
            if body[at] != "\n":
                out[at] = "\0"

    while i < len(body):
        if i == 0 or body[i - 1] == "\n":
            line_context[i] = context_now()
        kind, char = frames[-1][0], body[i]
        # Any hidden frame, not just the innermost: a quoted word inside `$( ... )`
        # is still the inner command's word rather than this step's.
        hidden = any(frame[0] in SHELL_HIDDEN for frame in frames)
        if hidden:
            blank(i, i + 1)
        if kind == "comment":
            if char == "\n":
                frames.pop()
                continue
        elif kind == "single":
            if char == "'":
                frames.pop()
                word_start = False
        elif kind == "backtick":
            if char == "\\":
                blank(i, i + 2)
                i += 1
            elif char == "`":
                frames.pop()
                word_start = False
        elif kind == "param":
            if char in "'\"`$\\{":
                raise unmodelled(f"`{char}` inside `${{ ... }}`")
            if char == "}":
                frames.pop()
                word_start = False
        elif char == "\\":
            if body.startswith("\\\n", i):
                joined = True
            else:
                word_start = joined = False
            if hidden:
                blank(i, i + 2)
            i += 2
            continue
        elif body.startswith("$(", i):
            frames.append(["comsub", 0, 0])
            word_start = True
            blank(i, i + 2)
            i += 2
            continue
        elif body.startswith("${", i):
            frames.append(["param", 0, 0])
            if hidden:
                blank(i, i + 2)
            i += 1
        elif char == "`":
            frames.append(["backtick", 0, 0])
            blank(i, i + 1)
        elif kind == "double":
            if char == '"':
                frames.pop()
                word_start = False
        elif char == "'":
            frames.append(["single", 0, 0])
        elif char == '"':
            frames.append(["double", 0, 0])
        elif char == "#" and word_start:
            if frames[-1][1] or frames[-1][2]:
                raise unmodelled("a `#` inside an unclosed `(` or `[`")
            frames.append(["comment", 0, 0])
            blank(i, i + 1)
        elif body.startswith("<<<", i):
            if hidden:
                blank(i, i + 3)
            i += 2
        elif body.startswith("<<", i):
            if frames[-1][1] or frames[-1][2]:
                raise unmodelled("`<<` inside an unclosed `(` or `[`")
            op = SHELL_HEREDOC.match(body, i)
            if op is None:
                raise unmodelled("a heredoc delimiter")
            strip, bare, single, double = op.groups()
            heredocs.append((bare or single or double, bool(strip), bare is None,
                             len(frames)))
            if hidden:
                blank(i, op.end())
            i = op.end() - 1
        elif char == "\n" and heredocs:
            if (frames[-1][1] or frames[-1][2]
                    or any(depth != len(frames) for *_, depth in heredocs)):
                raise unmodelled("a heredoc whose body starts inside another "
                                 "construct")
            i += 1
            for delimiter, strip, quoted, _ in heredocs:
                # Each body line is a line start of its own, so an unterminated
                # heredoc running to the end of the step still answers for every
                # line it swallowed rather than for none of them.
                while i < len(body):
                    line_context[i] = "a heredoc body"
                    end = body.find("\n", i) % (len(body) + 1)
                    line = body[i:end]
                    if (line.lstrip("\t") if strip else line) == delimiter:
                        i = end + 1
                        break
                    if not quoted and line.endswith("\\"):
                        raise unmodelled("a heredoc body line ending in `\\`")
                    blank(i, end)
                    i = end + 1
            heredocs.clear()
            word_start, joined = True, False
            continue
        elif kind == "comsub" and word_start and SHELL_CASE.match(body, i):
            raise unmodelled("`case` inside `$( ... )`")
        elif kind in ("code", "comsub") and char in "[]":
            frames[-1][2] = max(0, frames[-1][2] + (1 if char == "[" else -1))
        elif kind in ("code", "comsub") and char in "()":
            if frames[-1][2]:
                raise unmodelled("a parenthesis inside an unclosed `[`")
            frames[-1][1] += 1 if char == "(" else -1
            if frames[-1][1] < 0 and kind == "code":
                # A `case` pattern's `)` closes nothing.
                frames[-1][1] = 0
            elif frames[-1][1] < 0:
                frames.pop()
                word_start = False
                i += 1
                continue
        if kind in ("code", "comsub") and frames[-1][0] == kind:
            word_start, joined = char in SHELL_METACHARS, False
        i += 1

    return ShellStep(body, "".join(out), line_context, where)


def _spliced(text: str) -> tuple[str, list[int]]:
    """`text` as bash hands it to a command, and where each kept character was.

    A word's spelling is not what the command receives: bash removes quotes and
    escapes first, so `"bin""aries"=(` assigns `binaries`, `--argjs\\on` is the
    option `--argjson`, and `--argjson "fingerprint"` binds the same jq name as
    the bare spelling. A pattern read against the text as typed matches none of
    them, and for a gate that counts what it matches that is the silent
    direction: the spelling is not refused, it is not seen at all.

    The offsets are kept so a match can be reported and placed against the text
    as the step spells it.
    """
    origin: list[int] = []
    kept = 0
    for cut in SHELL_SPLICE.finditer(text):
        origin.extend(range(kept, cut.start()))
        kept = cut.end()
    origin.extend(range(kept, len(text)))
    return "".join(text[at] for at in origin), origin


def _consumer_word(var: str, suffix: str = "") -> str:
    """The loop variable as a command's word, in either expansion spelling."""
    name = re.escape(var)
    return rf'"\$(?:{name}|\{{{name}\}}){suffix}"'


def _loop_consumers(step: ShellStep, header: int, var: str) -> tuple[
        set[str], set[str], tuple[int, str] | None]:
    """Which of `PREVIEW_CONSUMERS` this loop's body runs on the loop variable.

    Returned beside that set: the same consumers found below the line that ended
    the countable part of the body, and that line as this step's own number and
    text, or None when nothing ended it. A body cut above its `sign` is missing
    the same member as a body with no `sign` anywhere in it, and a reader told to
    add the call is sent looking for a line already there -- so the caller has to
    name the cut, but only for a member the second set holds. A cut below the
    commands it was thought to suppress hid nothing, and naming it points at a
    line in a loop the refusal is not about. Carried out rather than acted on:
    which lines end a region, and which commands are counted before one does, are
    what they were.

    The body is the executable lines between the header and the `done` closing
    it, and the countable part of it ends at the first line that rebinds the
    variable: a consumer is counted for naming the variable, which says what it
    consumes only while the loop is what put the value there. A line the walk
    blanked carries no consumer either: its words are not this step's, so the
    argument bash would pass is not readable here and the command is not counted
    -- refusal by absence, the fail-closed direction. A consumer naming any other
    variable is not counted, because the array is only published by commands that
    name what the loop bound.

    The loop's own lines, not the code they reach: a command is read where it
    sits rather than by whether bash runs it, so a consumer under a false
    condition is counted like any other, and a call is read as the word it
    spells rather than as the body of the function behind it, so a helper that
    rebinds the variable rebinds nothing this reader sees. Decoys built either
    way are admitted. Rebinding is read the same way -- the variable is taken as
    rebound from the line that spells it onward, whether or not that line runs.

    A rebinding is read after quote removal and a consumer as the line spells it:
    the name a binding command takes is an argument bash unquotes first, while a
    consumer is recognised by the quoted expansion it passes and removing those
    quotes would leave no such word on any line.
    """
    binary = _consumer_word(var)
    signature = _consumer_word(var, r"\.minisig")
    rebind = re.compile(PREVIEW_REBIND.format(name=re.escape(var)))
    found: set[str] = set()
    cut_off: set[str] = set()
    ended: tuple[int, str] | None = None
    depth, binds, pending = 1, True, False
    for at in sorted(start for start in step.line_context if start > header):
        if step.context(at) is not None:
            continue
        end = step.raw.find("\n", at) % (len(step.raw) + 1)
        line = step.executed[at:end].rstrip("\0 \t")
        if PREVIEW_LOOP_CLOSE.match(line):
            depth -= 1
            if depth == 0:
                return found, cut_off, ended
            pending = False
            continue
        # Only the first: `binds` never goes back to True, so the lines after it
        # end nothing that is still open and naming one would point past the cut.
        if binds and any(spelling.group(1)
                         for spelling in rebind.finditer(_spliced(line)[0])):
            binds = False
            ended = (step.raw.count("\n", 0, at) + 1, step.raw[at:end].strip())
        joined = PREVIEW_LOOP_OPEN.match(line)
        if joined or (pending and PREVIEW_LOOP_DO.match(line)):
            depth += 1
        else:
            (found if binds else cut_off).update(
                name for name, pattern in PREVIEW_CONSUMERS
                if re.search(pattern.format(binary=binary,
                                            signature=signature), line))
        pending = bool(PREVIEW_LOOP_KEYWORD.match(line)) and not joined
    raise Refusal(f"{step.where} has a `for {var} in {PREVIEW_ARRAY_READ}; do` "
                  "loop with no `done` line closing it; which commands that loop "
                  "runs cannot be told, so neither can whether they are the ones "
                  "that sign and upload the array")


def _preview_artifacts(step: ShellStep) -> dict[str, str]:
    """The file name each triple is signed and uploaded under, from `binaries`.

    This reads the step's `run` text, not its execution: an assignment inside a
    branch, loop, function or pipeline still reads, a name built by expansion is
    not seen, and neither is anything outside that text that changes how bash
    runs it, such as a step `shell:` or a `BASH_ENV` startup file.
    """
    where, body = step.where, step.raw
    flat, origin = _spliced(body)
    for pattern, what in ((PREVIEW_ARRAY_JOIN, "a line continuation onto a "
                           "non-blank"), (PREVIEW_QUOTE_DECODE, "`$'...'` or "
                                          '`$"..."` quoting')):
        if found := pattern.search(body):
            line = body.count("\n", 0, found.start()) + 1
            raise Refusal(f"{where} uses {what} on line {line}; a name spelled "
                          "through it is not a name this gate can read, so "
                          "`binaries` could be assigned where it does not look")
    if rewrites := PREVIEW_REWRITE.search(flat):
        raise Refusal(f"{where} uses `{rewrites.group()}`, which makes bash rewrite "
                      "the step's lines before parsing them, so what the array "
                      "sits inside cannot be told from their text")
    if runs := PREVIEW_EVAL.search(flat):
        raise Refusal(f"{where} runs `{runs.group().strip()}`, which executes text "
                      "that is not a line of the step, so `binaries` could be "
                      "assigned where this gate does not read")

    opens: list[int] = []
    reads: list[int] = []
    loops: list[tuple[int, str]] = []
    for mention in PREVIEW_ARRAY_MENTION.finditer(flat):
        start = origin[mention.start()]
        line_start = body.rfind("\n", 0, start) + 1
        line = body[line_start:body.find("\n", start) % (len(body) + 1)]
        if PREVIEW_ARRAY_OPEN.fullmatch(line):
            opens.append(line_start)
        elif start >= 3 and body.startswith(PREVIEW_ARRAY_READ, start - 3):
            read_context = step.context(start)
            if read_context is not None:
                raise Refusal(f"{where} reads the array inside {read_context}, "
                              "where bash never runs it, so the loop this gate "
                              "read as walking the array walks nothing")
            header = PREVIEW_ARRAY_LOOP.fullmatch(line)
            if header is None:
                raise Refusal(f"{where} reads `{PREVIEW_ARRAY_READ}` on the line "
                              f"{line.strip()!r}, which is not spelled as a "
                              "header ending in `; do`; the array is the "
                              "authority for what is signed and "
                              "uploaded, so every executable read of it must be a "
                              f'`for <name> in {PREVIEW_ARRAY_READ}; do` header')
            reads.append(start)
            loops.append((line_start, header.group(1)))
        elif not PREVIEW_MANIFEST_OPEN.fullmatch(line):
            raise Refusal(f"{where} names `binaries` on the line {line.strip()!r}; "
                          "the array both loops walk may only be assigned by one "
                          "standalone `binaries=(` line and read as "
                          f"`{PREVIEW_ARRAY_READ}`, because any other spelling can "
                          "assign it where this gate does not read")
    # Exactly one: bash expands `binaries` to its latest assignment, so a second
    # array ahead of either loop changes what that loop signs or uploads while a
    # read of either array alone still passes.
    if len(opens) != 1:
        raise Refusal(f"{where} has no readable `binaries=( ... )` array (found "
                      f"{len(opens)}); the set every preview binary is signed and "
                      "uploaded from must be unambiguous, and a set this gate "
                      "cannot read is not an empty one")
    context = step.context(opens[0])
    if context is not None:
        raise Refusal(f"{where} has its `binaries=(` line inside {context}, so "
                      "bash never assigns the array from it and the array both "
                      "loops walk is not the one this gate would read")

    artifacts: dict[str, str] = {}
    for line in body[opens[0]:].rstrip("\n").split("\n")[1:]:
        if PREVIEW_ARRAY_CLOSE.fullmatch(line):
            break
        element = PREVIEW_ARRAY_LINE.fullmatch(line)
        if element is None:
            raise Refusal(f"{where} has the line {line.strip()!r} inside its "
                          "`binaries=( ... )` array, which is not one artifact "
                          "path; bash reads that line as something this gate "
                          "does not")
        triple, exe = element.groups()
        artifacts[triple] = f"phase-server-{triple}{exe or ''}"
    else:
        raise Refusal(f"{where} has no `)` line closing its `binaries=( ... )` "
                      "array")
    # A read ahead of the assignment walks an unset array, which `set -u` lets
    # expand to nothing: that loop signs or uploads no binary at all.
    if not reads or min(reads) < opens[0]:
        raise Refusal(f"{where} reads `{PREVIEW_ARRAY_READ}` ahead of its "
                      "`binaries=(` line, or nowhere; a loop that walks the array "
                      "before it is assigned signs and uploads nothing")
    consumed: set[str] = set()
    cuts: list[tuple[set[str], str]] = []
    for header_at, var in loops:
        names, cut_off, ended = _loop_consumers(step, header_at, var)
        consumed |= names
        if ended is not None:
            cuts.append((cut_off, f"line {ended[0]} names `{var}` other than as "
                                  f"an expansion of it ({ended[1]!r})"))
    if missing := [name for name, _ in PREVIEW_CONSUMERS if name not in consumed]:
        head = (f"{where} walks {PREVIEW_ARRAY_READ} but never "
                f"{', nor '.join(missing)} inside a loop over it")
        # Which member is missing does not say why, and a loop cut above the
        # command that would supply it reads as a loop that never had one. The
        # cut is what a reader has to see to find the line to edit -- but only
        # the cut that hid one of these members: any other names an innocent line
        # in place of the spelling that would supply what is missing.
        if named := [text for suppressed, text in cuts
                     if suppressed.intersection(missing)]:
            raise Refusal(
                f"{head}; {', and '.join(named)}, which ends the part of that "
                "loop counted here -- past that line the variable no longer names "
                "an element of the array, so the commands below it publish none "
                "of what the loop walks. Put the missing commands above that "
                "line; where it only reads the variable, spelling the name as an "
                "expansion of itself leaves it a use")
        raise Refusal(
            f"{head}; the array is the "
            "authority for what is published only when the commands that publish "
            "are the ones walking it, each naming the loop's own variable -- "
            '`sign "$<name>"`, `wrangler r2 object put ... --file "$<name>"` and '
            '`... --file "$<name>.minisig"`, in one loop or in several')
    return artifacts


def _loop_bodies(step: ShellStep) -> list[tuple[int, int]]:
    """Where every loop in the step opens and closes, by line start offset.

    A loop is what carries execution backwards. Position in the text is the
    order bash runs lines in only where nothing does that, so a line inside a
    loop runs again above every other line of the same body -- which is what a
    reader asking whether one line reaches another has to ask about, rather than
    which of the two is written first.

    Delimited exactly as a loop over the array is: a header ending in `do`, or a
    keyword line and a bare `do` beneath it, closed by the matching `done`. An
    unclosed loop is taken to run to the end of the step, which puts more of it
    inside a body rather than less.
    """
    open_at: list[int] = []
    bodies: list[tuple[int, int]] = []
    pending: int | None = None
    for at in sorted(step.line_context):
        if step.context(at) is not None:
            continue
        end = step.raw.find("\n", at) % (len(step.raw) + 1)
        line = step.executed[at:end].rstrip("\0 \t")
        if PREVIEW_LOOP_CLOSE.match(line):
            if open_at:
                bodies.append((open_at.pop(), at))
            pending = None
            continue
        joined = PREVIEW_LOOP_OPEN.match(line)
        if joined:
            open_at.append(at)
        elif pending is not None and PREVIEW_LOOP_DO.match(line):
            open_at.append(pending)
        pending = (at if PREVIEW_LOOP_KEYWORD.match(line) and not joined
                   else None)
    bodies.extend((at, len(step.raw)) for at in open_at)
    return bodies


def _upload_prefix(step: ShellStep) -> str:
    """The path every preview object is uploaded to, from the step's assignment.

    The value the uploads see, which is the last one assigned before they run. A
    line naming the name any way but as an expansion of itself is a value this
    gate did not read and bash uploads under -- `export prefix=...`,
    `printf -v prefix`, and the bare spelling with its value quoted some other
    way, none of which the one readable assignment matches.

    Whether such a line reaches an upload is execution order, not position: the
    expansions sit in the body of the loop that uploads, and bash runs that body
    again from the top, so a line below them rebinds the path every iteration
    after the first. A rebinding reaches an expansion when it is written above
    it, or when a loop holds them both. Below every expansion and inside no loop
    with one, nothing is left to upload under the value -- which is what leaves
    the `prefix=` URL parameter the step pages R2 with below its uploads naming
    no value read here.

    Read after quote removal, for the reason the loop variable is: every command
    binding the name takes it as an argument. The word bash reads as a name only
    when nothing in it is quoted is the assignment prefix, and one spelled that
    way is reported as the bare occurrence it is rather than modelled.

    The step's own text, not the code it reaches: a name built by expansion
    (`printf -v "pre${x}fix"`) is not seen, and neither is a rebinding in the
    body of a function, which is read as the word its call site spells.
    """
    names = re.compile(PREVIEW_REBIND.format(name="prefix"))
    assigned: list[str] = []
    reads: list[int] = []
    otherwise: list[tuple[int, str]] = []
    for at in sorted(step.line_context):
        if step.context(at) is not None:
            continue
        line = step.raw[at:step.raw.find("\n", at) % (len(step.raw) + 1)]
        spelled = list(names.finditer(_spliced(step.executed[at:at + len(line)])[0]))
        if not spelled:
            continue
        if (assignment := PREVIEW_PREFIX_ASSIGN.fullmatch(line)) is not None:
            assigned.append(assignment.group(1))
        elif any(spelling.group(1) for spelling in spelled):
            otherwise.append((at, line.strip()))
        else:
            reads.append(at)
    if len(assigned) != 1:
        raise Refusal(f"{step.where} has no readable "
                      f'`prefix="..."` assignment (found {len(assigned)}); the '
                      "path every preview object is uploaded to must be "
                      "unambiguous, and a path this gate supplies itself would "
                      "check the manifest against nothing")
    bodies = _loop_bodies(step)
    if live := [named for named in otherwise
                if any(named[0] < read
                       or any(start <= named[0] < end and start <= read < end
                              for start, end in bodies)
                       for read in reads)]:
        at, text = live[0]
        at_line = step.raw.count("\n", 0, at) + 1
        raise Refusal(f"{step.where} names `prefix` other than as an expansion of "
                      f"it on line {at_line} ({text!r}), which bash runs before a "
                      "line expanding it; bash uploads under the last value "
                      "assigned, so the path this gate read is not the one the "
                      "objects go to")
    return assigned[0]


def _manifest_command(step: ShellStep) -> tuple[int, int]:
    """Where the step's one `jq -n` command begins and ends.

    The manifest's two authorities -- the key map a desktop resolves against and
    the jq name its URLs are built from -- are read out of this region alone,
    each from its own part of it. Read from the whole step instead, either can be
    supplied by text that writes no manifest while the command that does writes
    something else.

    The command ends where bash ends it -- the first `;`, `&`, `|` or newline it
    does not carry as part of a word -- rather than at the next line, which is
    the same distinction the array's consumers are read with: a command sharing a
    line with this one owns the words after the separator, and a map echoed there
    is one no manifest carries. A word left open runs the end short, which is the
    fail-closed direction: the authorities are then not found and refuse.
    """
    flat, origin = _spliced(step.executed)
    starts: list[int] = []
    for found in PREVIEW_MANIFEST_JQ.finditer(flat):
        where = origin[found.start()]
        at = step.raw.rfind("\n", 0, where) + 1
        if step.context(at) is not None:
            continue
        if step.raw[at:where].strip():
            line = step.raw[at:step.raw.find("\n", at) % (len(step.raw) + 1)]
            raise Refusal(f"{step.where} runs `jq -n` on the line "
                          f"{line.strip()!r}, which is not where this gate can "
                          "tell the command begins; the manifest's keys and its "
                          "fingerprint are read from that command's own words")
        starts.append(at)
    if len(starts) != 1:
        raise Refusal(f"{step.where} has no readable `jq -n` command writing the "
                      f"manifest (found {len(starts)}); the object a desktop "
                      "resolves against is what one of them writes, and which "
                      "one cannot be told from the text")
    return starts[0], PREVIEW_COMMAND_TEXT.match(step.executed,
                                                 starts[0]).end()


def _jq_program(step: ShellStep, at: int, end: int) -> tuple[int, int]:
    """Where the program jq runs sits inside the command spanning `at`..`end`.

    jq emits the manifest from its program alone, and reads that program as the
    first word which is neither an option nor an option's value
    (`jq [options] <jq filter> [file...]`). Every other word of the command is
    one jq is handed and does not execute, so a map spelled in one -- the value
    of an `--arg`, a redirection target, an operand -- is a map no published
    manifest carries. Holding the map to the command instead leaves it satisfied
    by any word of it while the program the command really runs writes something
    else.

    Split into words as the step spells them and read one word at a time as bash
    hands it over: the boundaries are where the quotes are, and the option each
    word spells is what is left once they are removed. `"--argjson"` takes the
    two words after it exactly as the bare spelling does, and reading it as typed
    would make jq's filter the next word along.

    A word this gate cannot place refuses, and so does a redirection ahead of the
    program and a command with no program word at all -- the latter being what
    `--from-file` leaves, where the object a desktop resolves against is in a
    file and reading nothing is not reading an empty map. The step's own
    redirection sits past the program, which the walk has stopped at by then.
    """
    words = SHELL_WORD.finditer(step.executed, at, end)
    next(words, None)  # the command word, which begins the region
    skip, operands = 0, False
    for word in words:
        if skip:
            skip -= 1
            continue
        text = _spliced(word.group())[0]
        if text in SHELL_OPERATORS:
            line = step.raw.count("\n", 0, word.start()) + 1
            raise Refusal(f"{step.where} spells {text!r} on line {line}, ahead of "
                          "the program jq runs; bash hands neither it nor what "
                          "follows it to jq as a word, so which word is the "
                          "program cannot be told -- and a map spelled as a file "
                          "name is one no manifest carries")
        if operands or not text.startswith("-") or text == "-":
            return word.start(), word.end()
        if text == "--":
            operands = True
        elif text in JQ_VALUE_OPTIONS:
            skip = JQ_VALUE_OPTIONS[text]
        elif text not in JQ_FLAGS:
            line = step.raw.count("\n", 0, word.start()) + 1
            raise Refusal(f"{step.where} passes jq the option {text!r} on line "
                          f"{line}, which is not one this gate can tell takes a "
                          "value; which word jq reads as the program writing the "
                          "manifest depends on that, and the keys a desktop "
                          "resolves against are the ones that program emits")
    raise Refusal(f"{step.where} runs a `jq -n` command with no program word; the "
                  "manifest's keys are read out of the program jq runs, and a "
                  "command that carries none writes an object this gate never saw")


def preview_platforms() -> dict[str, set[str]]:
    """Every triple preview provisioning declares, one set per place it says so.

    `native_engine.rs` resolves a Preview key by looking the running host's
    `target_triple()` up in the signed manifest's `binaries` map, so each of these
    is a place a platform can be dropped while the release half stays green.

    They are kept apart rather than unioned, because a union is satisfied by any
    one spelling and the drift is precisely that they disagree: a triple built and
    signed but never written into the manifest leaves a desktop resolving nothing,
    and a union would still contain it. Held as subsets for the same reason the
    release assets are -- an extra triple strands no desktop, a missing one does.
    """
    build = _workflow_job(PREVIEW_WORKFLOW, PREVIEW_BUILD_JOB,
                          "the preview server binaries built per platform")
    matrix = (build.get("strategy") or {}).get("matrix")
    if not isinstance(matrix, dict):
        raise Refusal(f"{PREVIEW_WORKFLOW}: job '{PREVIEW_BUILD_JOB}' declares a "
                      f"strategy.matrix this gate cannot read ({matrix!r}); the "
                      "set of platforms preview builds cannot be read")
    if axes := sorted(set(matrix) - {"include", "exclude"}):
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_BUILD_JOB} declares matrix "
                      f"axes {axes} beside `include`; every combination of those "
                      "builds a preview server too, so the built set is larger "
                      "than the `include` list this gate reads")
    include = matrix.get("include")
    if not isinstance(include, list):
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_BUILD_JOB} declares no "
                      "strategy.matrix.include list; the built platform set "
                      "cannot be read from this shape")
    built: set[str] = set()
    for entry in include:
        if not isinstance(entry, dict) or not isinstance(entry.get("triple"), str):
            raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_BUILD_JOB} matrix has an "
                          f"entry with no readable `triple`: {entry!r}. That "
                          "field is the whole identity of a preview binary")
        built.add(entry["triple"])

    publish = _workflow_job(PREVIEW_WORKFLOW, PREVIEW_PUBLISH_JOB,
                            "the preview binaries downloaded, signed and published")
    steps = publish.get("steps")
    if not isinstance(steps, list):
        raise Refusal(f"{PREVIEW_WORKFLOW}: job '{PREVIEW_PUBLISH_JOB}' declares "
                      "no steps list; the job that signs and publishes preview "
                      "servers was reshaped")
    downloaded: set[str] = set()
    for step in steps:
        with_ = step.get("with") if isinstance(step, dict) else None
        name = with_.get("name") if isinstance(with_, dict) else None
        if not isinstance(name, str) or not name.startswith(PREVIEW_ARTIFACT_PREFIX):
            continue
        # The build job uploads under this same prefix as `${{ matrix.triple }}`,
        # which names every platform at once and so identifies none of them. Read
        # as a literal it would contribute one nonsense triple that no mapping
        # holds, turning a superset check into a permanent failure.
        if "${{" in name:
            continue
        downloaded.add(name.removeprefix(PREVIEW_ARTIFACT_PREFIX))

    bodies = _step_bodies(PREVIEW_WORKFLOW, PREVIEW_PUBLISH_JOB,
                          "the preview binaries signed and written to the manifest")
    body = _step_body(bodies, PREVIEW_SIGN_STEP, PREVIEW_WORKFLOW,
                      PREVIEW_PUBLISH_JOB,
                      "the signed binaries and the manifest naming them")
    # The array is the single authority for artifact file names: the step uploads
    # `basename "$binary"` taken from it, so the name in the manifest's URL has to
    # be exactly this one -- `.exe` included, which only windows carries.
    step = _shell_step(body, f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP}")
    artifacts = _preview_artifacts(step)
    signed = set(artifacts)
    writes_at, writes_end = _manifest_command(step)
    program_at, program_end = _jq_program(step, writes_at, writes_end)
    # The pairing is read from the words ahead of the program, which is where jq
    # takes its bindings: read from the whole command, the program's own text
    # supplies one, and a program is not what says a shell value reached jq.
    written = step.executed[writes_at:program_at]
    blocks = list(PREVIEW_MANIFEST_BLOCK.finditer(step.executed))
    if len(blocks) != 1:
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} has no readable "
                      f"`binaries: {{` object in the manifest it writes (found "
                      f"{len(blocks)}); the keys a desktop resolves against must "
                      "be unambiguous, and a set this gate cannot read is not an "
                      "empty one")
    block = blocks[0]
    # The one object in the step has to be the one that command writes: a map the
    # `jq -n` program does not carry is a map no desktop ever resolves against,
    # and reading it leaves the published manifest unchecked in full.
    if not writes_at <= block.start() < block.end() <= writes_end:
        line = step.raw.count("\n", 0, block.start()) + 1
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} spells its "
                      f"`binaries: {{` object on line {line}, outside the "
                      "`jq -n` command that writes the manifest; the keys a "
                      "desktop resolves against are the ones that command emits")
    if not program_at <= block.start() < block.end() <= program_end:
        line = step.raw.count("\n", 0, block.start()) + 1
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} spells its "
                      f"`binaries: {{` object on line {line}, in a word of that "
                      "`jq -n` command other than the program it runs; jq emits "
                      "the manifest from its program, so a map carried beside "
                      "one is a map no desktop ever resolves against")
    manifest_keys = PREVIEW_MANIFEST_KEY.findall(block.group(1))
    duplicate_keys = sorted({key for key in manifest_keys
                             if manifest_keys.count(key) > 1})
    if duplicate_keys:
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} writes duplicate "
                      f"manifest binary key(s) {duplicate_keys}; jq keeps the "
                      "later value, so every emitted URL pair must have one "
                      "unambiguous key")
    keys = set(manifest_keys)

    prefix = _upload_prefix(step)
    shell = re.search(r"\$(\w+)", prefix)
    if shell is None:
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} uploads to "
                      f"'{prefix}', which carries no variable; every fingerprint "
                      "would publish over one path, so the manifest could not "
                      "name a per-fingerprint object at all")
    jq_args = PREVIEW_JQ_ARG.findall(written)
    bound = [jq for jq, sh in jq_args if sh == shell.group(1)]
    if len(bound) != 1:
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} binds "
                      f"${shell.group(1)} to {len(bound)} jq argument(s) "
                      f"{sorted(bound)}; exactly one is what lets the manifest's "
                      "variable be checked against the uploaded path")
    # Exactly one binding of that jq name, by any option that binds one and
    # whatever value each carries: jq expands the name from only one of them and
    # which one is jq's own detail, so a second binding on either side leaves the
    # URLs checked against a value jq may not use. The options are named in the
    # refusal rather than counted, so the family that collided is readable from it.
    # Counted over the spliced text, so a second binding is counted by the words
    # jq is handed rather than by their spelling; the same step's array read has
    # already refused the constructs that would make splicing it unsound.
    rebound = [found.group(1)
               for found in PREVIEW_JQ_BIND.finditer(_spliced(step.executed)[0])
               if found.group(2) == bound[0]]
    if len(rebound) != 1:
        raise Refusal(f"{PREVIEW_WORKFLOW}: {PREVIEW_SIGN_STEP} has no readable "
                      f"`--arg {bound[0]}` binding (found {len(rebound)}: "
                      f"{sorted('--' + option for option in rebound)}); the "
                      "fingerprint every manifest URL names must be unambiguous, "
                      "and a second binding would check the upload path against a "
                      "value jq may not use")
    head, tail = prefix[:shell.start()], prefix[shell.end():]

    def expected(name: str) -> list[tuple[str, str]]:
        """The one URL that names the object this step uploads for `name`."""
        return [("op", "("), ("str", f"{PREVIEW_DATA_HOST}{head}"),
                ("op", "+"), ("var", bound[0]), ("op", "+"),
                ("str", f"{tail}/{name}"), ("op", ")")]

    # A desktop fetches `url` and `sig_url` verbatim, so an entry has to name the
    # object the step uploaded. Compared as tokens against a URL derived from that
    # step's own prefix, fingerprint binding and array-authorised name, because
    # every weaker rule leaves something free: a substring admits
    # `phase-server-<triple>-old`, a terminal segment admits a moved prefix, and a
    # comparison against a path spelled here admits a prefix that moves in the
    # workflow alone. Per key, so the report names which key rather than a count.
    paired: set[str] = set()
    for key in keys:
        entry = re.search(rf'"{re.escape(key)}":\s*\{{(.*?)\n\s*\}}',
                          block.group(1), re.S)
        if entry is None or key not in artifacts:
            continue
        urls = dict(PREVIEW_MANIFEST_URL.findall(entry.group(1)))
        name = artifacts[key]
        if (_jq_tokens(urls.get("url", "")) == expected(name)
                and _jq_tokens(urls.get("sig_url", ""))
                == expected(f"{name}.minisig")):
            paired.add(key)

    return {"builds": built, "downloads": downloaded, "signs": signed,
            "names in its manifest": keys,
            "gives a signed URL pair in its manifest": paired}


def main() -> int:
    try:
        mapped = mapped_platforms()
        published = published_platforms()
        attached = published_assets()
        provisioned = preview_platforms()
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

    triples = {triple for _, _, triple in mapped.values()}
    for what, declared in sorted(provisioned.items()):
        stranded = sorted(triples - declared)
        if not stranded:
            continue
        print(f"{MAPPING_SOURCE}'s ServerPlatform resolves {len(stranded)} "
              f"triple(s) that {PREVIEW_WORKFLOW} never {what}:", file=sys.stderr)
        for triple in stranded:
            print(f"  {triple}", file=sys.stderr)
        print("A desktop on that platform looks its triple up in the signed "
              "preview manifest and finds no binary, so Try Preview fails there "
              "while every release check stays green. Provision that triple, or "
              "stop resolving it.", file=sys.stderr)
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
    print(f"preview provisioning OK: {len(triples)} engine triple(s) "
          f"({', '.join(sorted(triples))}) each built, downloaded, signed, named "
          f"in {PREVIEW_WORKFLOW}'s manifest, and given a signed URL pair there")
    return 0


if __name__ == "__main__":
    sys.exit(main())
