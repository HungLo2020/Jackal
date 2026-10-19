#!/usr/bin/env python3
"""Reproduce all six stages’ palette-cycle PNG swatches and timing evidence offline.

Optional research tool. Historical code/data are restored only into a temporary
workspace. Cargo build/run never invokes this script or requires Git/Python.
"""
import argparse
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[2]
HERE = Path(__file__).resolve().parent
REFERENCE = 'e8fc3391d5b27fcd98b82fd27984141840449708'
FILES = ['Cargo.toml','Cargo.lock','src/audio.rs','src/game.rs','src/video.rs',
         'assets/game-data.bin','generated/native_game.rs']

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check',action='store_true',help='Compare exports without modifying repository assets')
    parser.add_argument('--output',type=Path,default=ROOT,help='Project root for assets/ and tests/fixtures/')
    args=parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='jackal-palette-cycle-reference-') as temp:
        reference=Path(temp)/'reference'
        exported=Path(temp)/'exported'
        for name in FILES:
            path=reference/name
            path.parent.mkdir(parents=True,exist_ok=True)
            path.write_bytes(subprocess.check_output(['git','show',f'{REFERENCE}:{name}'],cwd=ROOT))
        manifest=reference/'Cargo.toml'
        manifest.write_text(manifest.read_text().replace('rodio = "0.20.1"','rodio = "0.20.1"\nimage = { version = "0.25.10", default-features = false, features = ["png"] }\nserde_json = "1.0"'))
        with (reference/'src/video.rs').open('a') as file:
            file.write('\n'.join(line for line in (HERE/'video_export_methods.rs').read_text().splitlines() if 'let palette:' not in line and 'palette.json' not in line))
        with (reference/'src/video.rs').open('a') as file:
            file.write('\nimpl Video { pub fn palette_rgb(&self)->Vec<u32>{(0..32).map(|i|self.color(0x3f00+i,0)).collect()} }\n')
        native=reference/'generated/native_game.rs'
        native.write_text(native.read_text().replace('0xdde6=>{', '0xdde6=>{ crate::PALETTE_QUEUE.store(s.memory[0x21], std::sync::atomic::Ordering::Relaxed);'))
        shutil.copyfile(HERE/'palette_cycle_export_main.rs',reference/'src/main.rs')
        env=dict(os.environ,CARGO_TARGET_DIR=str(ROOT/'target/reference-palettes'))
        subprocess.run(['cargo','run','--offline','--manifest-path',str(manifest),'--',str(exported)],cwd=reference,env=env,check=True)
        differences=[]
        for source in sorted(exported.rglob('*')):
            if not source.is_file():continue
            relative=source.relative_to(exported)
            if relative.parts[0] == 'scratch': continue
            target=args.output/relative
            if args.check:
                if not target.exists() or hashlib.sha256(source.read_bytes()).digest()!=hashlib.sha256(target.read_bytes()).digest():differences.append(str(relative))
            else:
                target.parent.mkdir(parents=True,exist_ok=True)
                shutil.copyfile(source,target)
        if differences:raise SystemExit('Graphics differ: '+', '.join(differences))
        print('Stage palette-cycle PNGs and timing evidence verified' if args.check else 'Stage palette-cycle assets exported')

if __name__=='__main__':main()
