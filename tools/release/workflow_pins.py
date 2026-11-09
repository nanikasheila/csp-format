"""Check the GitHub workflows: what they run is fixed, and what they may do is said job by job.

An Action named by a tag (`@v7`) is whatever its owner points the tag at that
day, and it runs with this repository's token (in the release, one that may
write). So every Action is named by its commit, with the version beside it
for people (Dependabot raises both together), and a workflow as a whole gets
no right to write: the job that needs one says so.

    python tools/release/workflow_pins.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
USES = re.compile(r"^\s*(?:- )?uses:\s*(\S+)(.*)$", re.M)
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")
VERSION = re.compile(r"^\s+# v\d+(\.\d+)*$")
OWN_PERMISSIONS = re.compile(r"^permissions:(.*)\n((?: {2}.*\n)*)", re.M)


def problems(name: str, text: str) -> list[str]:
    found = []
    for used, note in USES.findall(text):
        if used.startswith("./"):
            continue
        if not PINNED.match(used) or not VERSION.match(note):
            found.append(f"{name}: {used}{note} (name the Action by its commit, with `# vX.Y.Z` beside it)")
    own = OWN_PERMISSIONS.search(text)
    if own is None:
        found.append(f"{name}: no `permissions:` of its own (the repository's default would apply)")
    elif re.search(r":\s*write\b", own.group(1) + own.group(2)):
        found.append(f"{name}: the workflow as a whole may write; give that right to the job that needs it")
    return found


def main() -> int:
    found = []
    for path in sorted((ROOT / ".github" / "workflows").glob("*.yml")):
        found += problems(path.name, path.read_text(encoding="utf-8"))
    for line in found:
        print(line)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
