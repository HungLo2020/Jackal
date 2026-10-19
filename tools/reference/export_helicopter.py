#!/usr/bin/env python3
"""Regenerate original rescue helicopter behavior fixtures; optional analysis only."""
import argparse, os, subprocess, tempfile
from pathlib import Path
from export_graphics import ROOT, HERE, REFERENCE, FILES
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--check', action='store_true')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='jackal-helicopter-reference-') as temp:
    root = Path(temp)
    for name in FILES:
        target = root / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(subprocess.check_output(['git', 'show', f'{REFERENCE}:{name}'], cwd=ROOT))
    manifest = root / 'Cargo.toml'
    manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"', 'rodio = "0.20.1"\nserde_json = "1.0"'))
    with (root/'src/game.rs').open('a') as stream:
        stream.write((HERE/'enemy_sequences_methods.rs').read_text())
    generated = root/'generated/native_game.rs'
    generated.write_text(generated.read_text().replace('0xc36b=>{', '0xc36b=>{ crate::record_sound(s.a);'))
    (root/'src/main.rs').write_text((HERE/'helicopter_export_main.rs').read_text())
    (root/'tests/fixtures').mkdir(parents=True)
    subprocess.run(['cargo', 'run', '--offline', '--manifest-path', str(manifest), '--', str(root)], check=True, env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-enemies')))
    relative = 'tests/fixtures/rescue_helicopter.json'
    result = (root/relative).read_bytes()
    if args.check:
        if result != (ROOT/relative).read_bytes(): raise SystemExit('Helicopter reference differs')
        print('Helicopter reference verified')
    else: (ROOT/relative).write_bytes(result)
