#!/usr/bin/env python3
"""Generate the Reference's data from the app, or check it (specs/website.md, W-REF-03, W-REF-04).

Usage: python3 reference.py            write site/data/reference.json
       python3 reference.py --check site/public

The Rust program in site/reference/ prints every command of the app's registry with its keys in
the default keymap, every window's own keys, the settings and the themes. This script gives each
an id, splits each key sequence into the presses it is made of, and places each on the Reference
page site/reference/topics.toml names. It fails on anything the map does not place and on a name
in the map the app does not have. With --check it writes nothing: it fails when the committed
data differs from what the app gives now, or when a built Reference page lacks a row's anchor.
"""

import json
import os
import re
import subprocess
import sys
import tomllib
from html.parser import HTMLParser
from pathlib import Path

SITE = Path(__file__).resolve().parent.parent
DATA = SITE / "data" / "reference.json"
TOPICS = SITE / "reference" / "topics.toml"

# Keys the keymap writes as words; any other run of characters without a space or a modifier is
# one press per character, as `gg` and `nP` are.
NAMED = {"Esc", "Enter", "Tab", "Space", "Backspace", "Delete", "Insert", "Home", "End",
         "PgUp", "PgDn"}

# The cheat sheet's command groups, in the order the app's shortcuts panel lists them.
GROUPS = ["Moving", "Selection", "Going", "Editing and Creating", "Completing, Dates and Tags",
          "Structure", "View and Panels"]

# The windows whose own keys the cheat sheet lists after the command groups.
CHEATSHEET_WINDOWS = ["Attachments Panel", "Picker", "Edit Dialog", "Date Prompt", "Text"]


def app_data():
    """What the app gives, from site/reference/, built in site/.cache/."""
    out = subprocess.run(
        ["cargo", "run", "--quiet", "--release", "--locked",
         "--manifest-path", str(SITE / "reference" / "Cargo.toml")],
        check=True, capture_output=True, text=True,
        env={**os.environ, "CARGO_TARGET_DIR": str(SITE / ".cache" / "target")},
    ).stdout
    return json.loads(out)


def slug(text):
    return re.sub(r"[^a-z0-9]+", "-", text.lower().replace("…", "")).strip("-")


def presses(label):
    """The presses of one key sequence as the keymap writes it: `Ctrl+W h` is Ctrl+W then h,
    `gg` is g then g, `Shift+↓` is one press."""
    if " " in label:
        return [p for part in label.split(" ") for p in presses(part)]
    if label in NAMED or len(label) == 1 or "+" in label[:-1]:
        return [label]
    return list(label)


def build(app, topics):
    problems = []
    places = {"commands": topics.get("commands", {}), "windows": topics.get("windows", {}),
              "settings": topics.get("settings", {})}

    def topic(kind, name):
        found = places[kind].get(name)
        if found is None:
            problems.append(f"topics.toml places no {kind[:-1]} {name!r}; add it to [{kind}]")
        return found

    titles = [c["title"] for c in app["commands"]]
    for title in sorted({t for t in titles if titles.count(t) > 1}):
        problems.append(f"the registry has two commands titled {title!r}")
    commands = [{
        "id": "key-" + slug(c["title"]),
        "title": c["title"],
        "group": c["group"],
        "context": c["context"],
        "topic": topic("commands", c["title"]),
        "keys": [presses(k) for k in c["keys"]],
    } for c in app["commands"]]
    windows = [{
        "id": "keys-" + slug(w["window"]),
        "title": w["window"],
        "topic": topic("windows", w["window"]),
        "keys": [{"title": k["title"], "keys": [presses(s) for s in k["keys"]]} for k in w["keys"]],
    } for w in app["windows"]]
    settings = [{**s, "id": "setting-" + s["key"].replace("_", "-"),
                 "topic": topic("settings", s["key"])} for s in app["settings"]]
    for kind, have in (("commands", titles), ("windows", [w["window"] for w in app["windows"]]),
                       ("settings", [s["key"] for s in app["settings"]])):
        for name in places[kind]:
            if name not in have:
                problems.append(f"topics.toml places {kind[:-1]} {name!r}, which the app does not have")
    if problems:
        sys.exit("reference.py:\n  " + "\n  ".join(problems))

    groups = {title: [] for title in GROUPS}
    for c in commands:
        if c["keys"] and c["context"] != "conflict":
            if c["group"] not in groups:
                sys.exit(f"reference.py: the cheat sheet has no place for the group {c['group']!r}")
            groups[c["group"]].append({"title": c["title"], "keys": c["keys"]})
    cheatsheet = {
        "groups": [{"title": title, "rows": rows} for title, rows in groups.items()],
        "windows": [w for name in CHEATSHEET_WINDOWS for w in windows if w["title"] == name],
        "unbound": [c["title"] for c in commands if not c["keys"]],
    }
    return {"commands": commands, "windows": windows, "settings": settings,
            "themes": app["themes"], "cheatsheet": cheatsheet}


class Ids(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids = set()

    def handle_starttag(self, tag, attrs):
        self.ids.update(value for name, value in attrs if name == "id" and value)


def check(data, public):
    committed = json.loads(DATA.read_text()) if DATA.exists() else None
    problems = []
    if committed != data:
        problems.append(f"{DATA.relative_to(SITE.parent)} differs from what the app gives now; "
                        "run `bash site/site.sh reference`")
    rendered = {}
    for page in (public / "reference").rglob("index.html"):
        if page.parent.name == "cheatsheet":
            continue
        parser = Ids()
        parser.feed(page.read_text())
        rendered[page.parent.name] = parser.ids
    for kind in ("commands", "windows", "settings"):
        for row in data[kind]:
            if row["id"] not in rendered.get(row["topic"], set()):
                problems.append(f"the Reference page {row['topic']!r} does not render "
                                f"{kind[:-1]} {row['title' if kind != 'settings' else 'key']!r} "
                                f"(no #{row['id']})")
    if problems:
        sys.exit("reference.py --check:\n  " + "\n  ".join(problems))
    print(f"reference: {len(data['commands'])} commands, {len(data['windows'])} windows and "
          f"{len(data['settings'])} settings, all rendered")


def main():
    data = build(app_data(), tomllib.loads(TOPICS.read_text()))
    if sys.argv[1:2] == ["--check"]:
        check(data, Path(sys.argv[2]).resolve())
    else:
        DATA.parent.mkdir(exist_ok=True)
        DATA.write_text(json.dumps(data, indent=1, ensure_ascii=False) + "\n")
        print(DATA)


if __name__ == "__main__":
    main()
