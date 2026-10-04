#!/usr/bin/python3
"""Add LinuxDrift without replacing the user's list or selected index."""
import argparse
import os
from pathlib import Path
import re
import shutil
import tempfile

ENTRY = '  "LinuxDrift"  linuxdrift --root'


def register(text, defaults=""):
    # Migrate the local pre-release name in place, preserving selected indices.
    text = re.sub(r"(?m)^.*\blinuxflux\s+--?root.*$",
                  lambda match: match.group().replace("linuxflux", "linuxdrift").replace("LinuxFlux", "LinuxDrift"), text)
    if re.search(r"\blinuxdrift\s+(?:--?root|--window-id)", text):
        return text
    lines = text.splitlines(keepends=True)
    start = next((i for i, line in enumerate(lines) if re.match(r"^programs\s*:", line)), None)
    if start is None:
        source = defaults.splitlines(keepends=True)
        first = next((i for i, line in enumerate(source) if re.match(r"^\*programs\s*:", line)), None)
        block = "programs:\n"
        if first is not None:
            last = first
            while source[last].rstrip().endswith("\\") and last + 1 < len(source):
                last += 1
            block = "".join(source[first:last + 1]).removeprefix("*")
        return register(text.rstrip() + "\n\n" + block, "")
    end = start
    while lines[end].rstrip().endswith("\\") and end + 1 < len(lines):
        end += 1
    # Append to the resource, preserving program order (selected is an index).
    lines[end] = lines[end].rstrip() + " \\n\\\n"
    lines.insert(end + 1, ENTRY + "\n")
    return "".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--file", type=Path, default=Path.home() / ".xscreensaver")
    args = parser.parse_args()
    path = args.file
    if path.is_symlink():
        raise SystemExit("Refusing to replace a symlink: " + str(path))
    previous = path.read_text() if path.exists() else ""
    defaults_path = Path("/etc/X11/app-defaults/XScreenSaver")
    defaults = defaults_path.read_text() if defaults_path.exists() else ""
    updated = register(previous, defaults)
    if previous == updated:
        return
    backup = path.with_name(path.name + ".before-linuxdrift")
    if path.exists() and not backup.exists():
        shutil.copy2(path, backup)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, temporary = tempfile.mkstemp(prefix=".linuxdrift-", dir=path.parent)
    try:
        with os.fdopen(fd, "w") as file:
            file.write(updated)
            file.flush()
            os.fsync(file.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
    print("LinuxDrift registered in " + str(path))


if __name__ == "__main__":
    main()
