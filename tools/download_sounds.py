#!/usr/bin/env python3
"""
Download BAR sound effects from the Beyond All Reason repository.
Caches downloads in tools/.cache/ and outputs WAV files to assets/sounds/.
"""

import os
import urllib.request
from pathlib import Path

BAR_BASE = "https://raw.githubusercontent.com/beyond-all-reason/Beyond-All-Reason/master/sounds"

# (local_name, remote_path)
SOUNDS = [
    ("lasrfir1",            "weapons/lasrfir1.wav"),
    ("flashemg",            "weapons/flashemg.wav"),
    ("cannon1",             "weapons/cannon1.wav"),
    ("cannhvy1",            "weapons/cannhvy1.wav"),
    ("lasrfir3",            "weapons/lasrfir3.wav"),
    ("disigun1",            "weapons/disigun1.wav"),
    ("xplosml2",            "weapons/xplosml2.wav"),
    ("xplomed2",            "weapons/xplomed2.wav"),
    ("xplolrg3",            "weapons/xplolrg3.wav"),
    ("cmd-move-short",      "commands/cmd-move-short.wav"),
    ("cmd-attack",          "commands/cmd-attack.wav"),
    ("cmd-build",           "commands/cmd-build.wav"),
    ("cmd-reclaim",         "commands/cmd-reclaim.wav"),
    ("cmd-default-select",  "commands/cmd-default-select.wav"),
    ("unitready",           "replies/unitready.wav"),
    ("build2",              "replies/build2.wav"),
    ("nanlath1",            "weapons/nanlath1.wav"),
]

def main():
    script_dir = Path(__file__).parent
    cache_dir = script_dir / ".cache" / "sounds"
    output_dir = script_dir.parent / "assets" / "sounds"

    cache_dir.mkdir(parents=True, exist_ok=True)
    output_dir.mkdir(parents=True, exist_ok=True)

    for local_name, remote_path in SOUNDS:
        out_file = output_dir / f"{local_name}.wav"
        cache_file = cache_dir / f"{local_name}.wav"

        if out_file.exists():
            print(f"  [skip] {local_name}.wav (already exists)")
            continue

        if cache_file.exists():
            print(f"  [cache] {local_name}.wav")
            data = cache_file.read_bytes()
        else:
            url = f"{BAR_BASE}/{remote_path}"
            print(f"  [download] {local_name}.wav <- {url}")
            try:
                data = urllib.request.urlopen(url).read()
                cache_file.write_bytes(data)
            except Exception as e:
                print(f"    ERROR: {e}")
                continue

        out_file.write_bytes(data)
        print(f"    -> {out_file}")

    print(f"\nDone! {len(SOUNDS)} sounds -> {output_dir}")

if __name__ == "__main__":
    main()
