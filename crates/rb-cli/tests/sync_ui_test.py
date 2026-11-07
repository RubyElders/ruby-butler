#!/usr/bin/env python3
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tarfile
import time

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    'progress_ui', Path(__file__).resolve().parents[2] / 'rb-progress/tests/pty_ui_test.py'
)
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)

binary = str(Path(sys.argv[1]).resolve())

def verify_automatic_ruby_install(bundle_source):
    archive = io.BytesIO()
    ruby_source = (Path(__file__).parent / 'fixtures/ruby_install/ruby.sh').read_text()
    ruby_source = ruby_source.replace('elif [ "$1" = "-rrbconfig" ]; then',
                                      'elif [ "$1" = "-rrbconfig" ]; then\n    sleep 0.3')
    with tarfile.open(fileobj=archive, mode='w:gz') as tar:
        for path, contents, mode in [
                ('bin/ruby', ruby_source, 0o755),
                ('bin/bundle', bundle_source, 0o755),
                ('lib/ruby/fixture.rb', '', 0o644)]:
            data = contents.encode()
            entry = tarfile.TarInfo('rv-ruby@3.4.10/3.4.10/' + path)
            entry.size, entry.mode = len(data), mode
            tar.addfile(entry, io.BytesIO(data))
    data = archive.getvalue()
    digest = hashlib.sha256(data).hexdigest()
    original_environment = os.environ.copy()
    original_directory = os.getcwd()
    try:
        for key in ['RB_RUBY_VERSION', 'RB_RUBIES_DIR', 'RB_WORK_DIR', 'RB_PROJECT',
                    'RB_AUTO_INSTALL_RUBY', 'RB_NO_BUNDLER', 'BUNDLE_GEMFILE']:
            os.environ.pop(key, None)
        for command in [['sync'], ['x', 'rails', 'c']]:
            for rows in [8, 24]:
                with tempfile.TemporaryDirectory() as temporary:
                    root = Path(temporary)
                    os.chdir(root)
                    (root / 'config.toml').write_text('')
                    (root / '.ruby-version').write_text('3.4.10\n')
                    (root / 'Gemfile').write_text("source 'https://rubygems.org'\n")
                    cache = root / 'cache/rb/rubies'
                    (cache / 'archives').mkdir(parents=True)
                    (cache / 'archives' / (digest + '.tar.gz')).write_bytes(data)
                    (cache / 'catalog.json').write_text(json.dumps({
                        'url': 'https://api.github.com/repos/spinel-coop/rv-ruby/releases/latest',
                        'etag': None,
                        'fetched': {'secs_since_epoch': int(time.time()), 'nanos_since_epoch': 0},
                        'release': {'tag_name': 'fixture', 'assets': [{
                            'name': 'ruby-3.4.10.x86_64_linux.tar.gz',
                            'size': len(data), 'digest': 'sha256:' + digest,
                            'browser_download_url': 'https://example.invalid/ruby.tar.gz'}]}}))
                    os.environ['XDG_CACHE_HOME'] = str(root / 'cache')
                    args = ['-c', str(root / 'config.toml'), '-R', str(root / 'rubies'),
                            '--package-backend', 'bundler'] + command
                    expanded = []
                    def observe(screen):
                        if screen.find('│  │  ├─ ● Validating installation') is not None:
                            expanded.append(True)
                    screen, output = ui.capture(binary, args, rows, 120, 0, observe)
                    assert expanded, 'automatic Ruby installation never appeared inside its parent'
                    assert screen.find('[shell]$ rb --db sync') == 0
                    if command == ['sync']:
                        assert screen.find('┌─ ✓ Preparing your bundle') == 1
                        assert screen.find('│  ├─ ✓ Installed Ruby 3.4.10 in ') == 2
                        assert sum('┌─' in screen.line(row) for row in range(rows)) == 1
                        assert not any('Installing Ruby' in screen.line(row) for row in range(rows))
                        end = screen.find('└─ ✓ Your bundle is ready')
                    else:
                        assert screen.find('✓ Your bundle is ready') == 1
                        assert screen.find('Rails console fixture') == 2
                        assert not any('Installed Ruby' in screen.line(row) for row in range(rows))
                        end = 2
                    assert end is not None
                    assert screen.row == end + 1 and screen.column == 0
                    assert all(screen.line(row) for row in range(end + 1))
                    assert not any(screen.line(row) for row in range(end + 1, rows))
                    assert b'Validated installation' in output
                    print(f'PASS automatic Ruby install command={command[0]} rows={rows}')
                    os.chdir(original_directory)
    finally:
        os.chdir(original_directory)
        os.environ.clear()
        os.environ.update(original_environment)


with tempfile.TemporaryDirectory() as temporary:
    root = Path(temporary)
    bin_dir = root / 'rubies/ruby-3.4.5/bin'
    bin_dir.mkdir(parents=True)
    (root / 'Gemfile').write_text("source 'https://rubygems.org'\n")
    bundle = bin_dir / 'bundle'
    bundle.write_text('''#!/bin/sh
printf '%s\\n' "$*" >> calls
case "$1" in
  exec) echo 'Rails console fixture'; exit 0 ;;
  check) sleep 0.3; test -f ready ;;
  lock)
    if [ -f lock-fail ]; then echo 'lock reconciliation failed' >&2; exit 1; fi
    if [ -f desired-lock ]; then cat desired-lock > Gemfile.lock; fi
    echo 'lockfile updated'; echo 'lock warning' >&2 ;;
  install)
    echo 'installing fixture'
    sleep 0.3
    i=0
    while [ "$i" -lt 3000 ]; do
      echo 'compiler diagnostic: testing concurrent stdout and stderr pipe drainage' >&2
      i=$((i + 1))
    done
    if [ -f fail ]; then echo 'compiler failed' >&2; exit 1; fi
    printf '%s\\n' 'preview one' 'preview two' 'preview three' >&2
    sleep 0.3
    echo 'bundle complete'
    ;;
esac
''')
    bundle.chmod(0o755)
    args = ['-R', str(root / 'rubies'), '--package-backend', 'bundler', 'sync']
    previous = os.getcwd()
    os.chdir(root)
    try:
        for ready in [False, True]:
            if ready:
                (root / 'ready').touch()
            (root / 'calls').write_text('')
            plain = subprocess.run([binary] + args, capture_output=True, timeout=20)
            assert plain.returncode == 0, plain.stderr
            summary = 'Everything is already in order' if ready else 'Your bundle is ready'
            assert plain.stdout == b'', plain.stdout
            assert b'[overall] Preparing your bundle' in plain.stderr
            assert b'[overall] complete: ' + summary.encode() in plain.stderr
            assert b'[worker: 0]' in plain.stderr
            assert b'\x1b[' not in plain.stdout + plain.stderr
            assert (b'lock warning' if ready else b'compiler diagnostic') in plain.stderr
            assert (b'lockfile updated' if ready else b'bundle complete') in plain.stderr
            expected = ['check', 'lock --local'] if ready else ['check', 'install']
            assert (root / 'calls').read_text().splitlines() == expected
            for rows in [8, 24]:
                preview_seen = []
                def observe(screen):
                    first = screen.find('│  │     preview one')
                    if first is not None and first + 2 < rows:
                        if (screen.line(first + 1).strip() == '│  │     preview two'
                                and screen.line(first + 2).strip() == '│  │     preview three'):
                            preview_seen.append(True)
                screen, output = ui.capture(binary, args, rows, 80, 0, observe)
                if not ready and rows == 24:
                    assert preview_seen, 'three-line Bundler preview was never displayed'
                    for line in ['preview one', 'preview two', 'preview three']:
                        assert ('\x1b[2m│  │     ' + line + '\x1b[22m').encode() in output

                assert screen.find('[shell]$ rb --db sync') == 0
                assert screen.find('┌─ ✓ Preparing your bundle') == 1
                end = screen.find('└─ ✓ ' + ('Everything is already in order' if ready else 'Your bundle is ready'))
                assert end is not None
                if ready:
                    assert any('checking lockfile 1/1 | unchanged' in screen.line(row) for row in range(rows))
                    assert b'updated lockfile' not in output
                for phase in ['checking dependencies', 'checking lockfile' if ready else 'installing dependencies']:
                    assert sum(phase in screen.line(row) for row in range(rows)) == 1
                assert screen.row == end + 1 and screen.column == 0
                assert not any(screen.line(row) for row in range(end + 1, rows))
                assert b'worker 0' in output
                assert b'checking dependencies' in output
                assert (b'checking lockfile' if ready else b'installing dependencies') in output
                assert b'\x1b[2J' not in output
                print(f'PASS sync PTY ready={ready} rows={rows}')
        for before, after, changed in [(None, 'first lock', True),
                                       ('old lock', 'new lock', True),
                                       ('same lock', 'same lock', False)]:
            lockfile = root / 'Gemfile.lock'
            if before is None:
                lockfile.unlink(missing_ok=True)
            else:
                lockfile.write_text(before)
            (root / 'desired-lock').write_text(after)
            (root / 'calls').write_text('')
            screen, output = ui.capture(binary, args, 24, 80, 0)
            assert (root / 'calls').read_text().splitlines() == ['check', 'lock --local']
            assert lockfile.read_text() == after
            detail = 'updated lockfile' if changed else 'unchanged'
            assert any('checking lockfile 1/1 | ' + detail in screen.line(row) for row in range(24))
            summary = 'Your bundle is ready' if changed else 'Everything is already in order'
            assert screen.find('└─ ✓ ' + summary) is not None
            if not changed:
                assert b'updated lockfile' not in output
        (root / 'desired-lock').unlink()
        (root / 'lock-fail').touch()
        (root / 'calls').write_text('')
        failed_lock = subprocess.run([binary] + args, capture_output=True, timeout=20)
        assert failed_lock.returncode != 0
        assert b'Bundle lock failed' in failed_lock.stdout + failed_lock.stderr
        assert (root / 'calls').read_text().splitlines() == ['check', 'lock --local']
        (root / 'lock-fail').unlink()
        print('PASS lockfile creation, changes, identical rewrites and failures')
        exec_args = ['-R', str(root / 'rubies'), '--package-backend', 'bundler', 'x', 'rails', 'c']
        for ready in [True, False]:
            if not ready:
                (root / 'ready').unlink()
            plain_exec = subprocess.run([binary] + exec_args, capture_output=True, timeout=20)
            assert plain_exec.returncode == 0, plain_exec.stderr
            assert plain_exec.stdout == b'Rails console fixture\n'
            assert b'\x1b[' not in plain_exec.stderr
            if ready:
                assert plain_exec.stderr == b''
            else:
                for message in [b'[overall] Preparing your bundle',
                                b'[worker: 0] installing dependencies',
                                b'[worker: 0] bundle complete',
                                b'[overall] complete: Your bundle is ready']:
                    assert message in plain_exec.stderr
            for rows in [8, 24]:
                (root / 'calls').write_text('')
                screen, output = ui.capture(binary, exec_args, rows, 80, 0)
                expected_row = 1 if ready else 2
                assert screen.find('Rails console fixture') == expected_row
                assert screen.row == expected_row + 1 and screen.column == 0
                assert not any(screen.line(row) for row in range(expected_row + 1, rows))
                calls = (root / 'calls').read_text().splitlines()
                assert calls == (['check', 'exec rails c'] if ready
                                 else ['check', 'check', 'install', 'exec rails c'])
                if ready:
                    assert b'Preparing your bundle' not in output
                    assert b'worker 0' not in output
                else:
                    assert screen.find('✓ Your bundle is ready') == 1
                    assert b'worker 0' in output
                    assert b'preview three' in output
                    assert not any('┌─' in screen.line(row) for row in range(rows))
                print(f'PASS exec PTY ready={ready} rows={rows}')
        (root / 'fail').touch()
        failed = subprocess.run([binary] + args, capture_output=True, timeout=20)
        assert failed.returncode != 0
        assert b'compiler failed' in failed.stdout + failed.stderr
        assert failed.stdout == b''
        assert b'[worker: 0] failed:' in failed.stderr
        (root / 'calls').write_text('')
        failed_exec = subprocess.run([binary] + exec_args, capture_output=True, timeout=20)
        assert failed_exec.returncode != 0
        assert b'compiler failed' in failed_exec.stdout + failed_exec.stderr
        assert 'exec rails c' not in (root / 'calls').read_text().splitlines()
        assert b'Rails console fixture' not in failed_exec.stdout
        print('PASS sync and exec failures prevent launching the command')
        verify_automatic_ruby_install(bundle.read_text())
    finally:
        os.chdir(previous)
