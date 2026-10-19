#!/usr/bin/env python3
"""Capture full original backgrounds and isolated in-scene turrets across all stages.

Research-only reference execution; cargo run/test uses the committed PNG/JSON fixtures.
"""
import argparse
import os
from pathlib import Path
import subprocess
import tempfile
from export_graphics import ROOT, HERE, REFERENCE, FILES

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='jackal-scene-alignment-') as temporary:
    root = Path(temporary)
    for name in FILES:
        target = root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(subprocess.check_output(['git', 'show', f'{REFERENCE}:{name}'], cwd=ROOT))
    manifest = root / 'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"', 'rodio = "0.20.1"\nserde_json = "1.0"\nimage = { version = "0.25.10", default-features = false, features = ["png"] }'))
    with (root / 'src/video.rs').open('a') as stream:
        stream.write((HERE / 'video_export_methods.rs').read_text())
    with (root / 'src/game.rs').open('a') as stream:
        stream.write('\nimpl Game {pub fn draw_alignment(&mut self)->Result<(),String>{self.bank=1;self.memory[0x27]=1;self.push_word(0xc28e);self.block=0x8008;self.run_until_idle()}}\n')
    (root / 'src/main.rs').write_text((HERE / 'scene_alignment_main.rs').read_text())
    output = root / 'output'
    subprocess.run(['cargo', 'run', '--offline', '--manifest-path', str(manifest), '--', str(output)], check=True, env=dict(os.environ, CARGO_TARGET_DIR=str(ROOT / 'target/reference-graphics')))
    destination = ROOT / 'tests/fixtures/scene_alignment'
    destination.mkdir(parents=True, exist_ok=True)
    output_names = {source.name for source in output.iterdir()}
    stale = [target for target in destination.glob('*.png') if target.name not in output_names]
    if args.check and stale:
        raise SystemExit(f'Stale scene fixtures: {[target.name for target in stale]}')
    for target in stale:
        target.unlink()
    for source in sorted(output.iterdir()):
        target = destination / source.name
        if args.check:
            if not target.exists() or target.read_bytes() != source.read_bytes():
                raise SystemExit(f'Original scene fixture differs: {source.name}')
        else:
            target.write_bytes(source.read_bytes())
    print('Original scene alignment fixtures verified' if args.check else 'Original scene alignment fixtures exported')
