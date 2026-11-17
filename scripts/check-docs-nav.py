"""Fails when a page under docs/ is missing from the nav of mkdocs.yml, or the
nav names a page docs/ does not have (DOC-05). Zensical's strict build reports
neither, and a page left out of the nav cannot be reached from the site."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def navigation(config: str) -> list[str]:
    """Return the pages the nav names: the last word of each item that ends in .md."""
    found: list[str] = []
    in_nav = False
    for line in config.splitlines():
        if line and not line.startswith((" ", "-")):
            in_nav = line == "nav:"
            continue
        item = line.strip()
        if in_nav and item.startswith("- "):
            word = item.rsplit(" ", 1)[-1]
            if word.endswith(".md"):
                found.append(word)
    return found


def main() -> int:
    """Compare the pages with the nav and report each difference."""
    docs = ROOT / "docs"
    pages = sorted(path.relative_to(docs).as_posix() for path in docs.rglob("*.md"))
    named = navigation((ROOT / "mkdocs.yml").read_text(encoding="utf-8"))
    status = 0
    if not named:
        print("check-docs-nav: mkdocs.yml has no nav", file=sys.stderr)
        return 1
    for page in pages:
        if page not in named:
            print(f"check-docs-nav: the nav does not name docs/{page}", file=sys.stderr)
            status = 1
    for page in named:
        if page not in pages:
            print(f"check-docs-nav: the nav names docs/{page}, which does not exist",
                  file=sys.stderr)
            status = 1
    return status


if __name__ == "__main__":
    sys.exit(main())
