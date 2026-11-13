"""Package and smoke-check the Chinese Apple Silicon build on a macOS host."""
import hashlib
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
VERSION = '2.31.2_1'
NAME = f'AG_{VERSION}_macos'


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True, stderr=subprocess.STDOUT)


def main():
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise SystemExit('Run this packager on an Apple Silicon Mac.')
    binary = ROOT / 'target/aarch64-apple-darwin/release/ag_unlocker'
    description = run('file', str(binary))
    assert 'Mach-O 64-bit executable arm64' in description, description
    print(description, end='')
    print(run('codesign', '--force', '--sign', '-', str(binary)), end='')
    print(run('codesign', '--verify', '--strict', '--verbose=2', str(binary)), end='')
    print(run('otool', '-L', str(binary)), end='')
    load_commands = run('otool', '-l', str(binary))
    assert 'minos 13.0' in load_commands, 'Unexpected macOS deployment target'
    banner = run(str(binary), '--version')
    assert '2.31.2' in banner, banner

    dist = ROOT / 'dist'
    dist.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='ag-macos-') as temporary:
        bundle = Path(temporary) / NAME
        bundle.mkdir()
        shutil.copy2(binary, bundle / 'ag_unlocker')
        (bundle / 'ag_unlocker').chmod(0o755)
        for source, destination in [
            ('macos/README.md', '使用说明.md'),
            ('README.md', 'README.md'),
            ('LICENSE', 'LICENSE.txt'),
            ('assets/fonts/OFL.txt', 'FONT-LICENSE.txt'),
        ]:
            shutil.copy2(ROOT / source, bundle / destination)
        launcher = bundle / '启动.command'
        launcher.write_text(
            '#!/bin/sh\nset -eu\ncd -- "$(dirname -- "$0")"\nexec ./ag_unlocker "$@"\n',
            encoding='utf-8', newline='\n')
        launcher.chmod(0o755)
        (bundle / 'SOURCE.txt').write_text(
            'https://github.com/sjgld/Antigravity\n'
            + run('git', 'rev-parse', 'HEAD').strip() + '\n'
            + f'Version: {VERSION}\nTarget: aarch64-apple-darwin\nMinimum macOS: 13.0\n',
            encoding='utf-8')
        subprocess.run(['sh', '-n', str(launcher)], check=True)
        archive = dist / f'{NAME}.tar.gz'
        with tarfile.open(archive, 'w:gz') as output:
            output.add(bundle, arcname=NAME)

        # Exercise the files that actually ship, including archive execute bits.
        extracted = Path(temporary) / 'extracted'
        with tarfile.open(archive, 'r:gz') as source:
            source.extractall(extracted, filter='data')
        shipped = extracted / NAME
        assert os.access(shipped / 'ag_unlocker', os.X_OK)
        assert os.access(shipped / '启动.command', os.X_OK)
        print(run('codesign', '--verify', '--strict', '--verbose=2', str(shipped / 'ag_unlocker')), end='')
        assert '2.31.2' in run(str(shipped / '启动.command'), '--version')
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (dist / 'SHA256SUMS-macos.txt').write_text(
            f'{digest}  {archive.name}\n', encoding='utf-8')
        print(f'Packaged and verified: {archive.name} ({archive.stat().st_size} bytes)')
        print(f'SHA256: {digest}')


if __name__ == '__main__':
    main()
