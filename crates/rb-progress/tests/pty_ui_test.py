#!/usr/bin/env python3
import codecs
import fcntl
import os
import pty
import select
import struct
import subprocess
import termios
import unicodedata
import sys
import time


class Screen:
    def __init__(self, rows, columns):
        self.rows = rows
        self.columns = columns
        self.cells = [[' '] * columns for _ in range(rows)]
        self.row = 0
        self.column = 0
        self.pending = b''
        self.decoder = codecs.getincrementaldecoder('utf-8')()

    def newline(self):
        self.row += 1
        self.column = 0
        if self.row >= self.rows:
            self.cells.pop(0)
            self.cells.append([' '] * self.columns)
            self.row = self.rows - 1

    def write(self, value):
        for char in self.decoder.decode(value):
            if char == '\r':
                self.column = 0
            elif char == '\n':
                self.newline()
            elif char.isprintable():
                width = 0 if unicodedata.combining(char) else 2 if unicodedata.east_asian_width(char) in ('W', 'F') else 1
                if width == 0:
                    previous = self.column - 1
                    while previous > 0 and self.cells[self.row][previous] == '':
                        previous -= 1
                    if previous >= 0:
                        self.cells[self.row][previous] += char
                    continue
                if self.column + width > self.columns:
                    self.newline()
                self.cells[self.row][self.column] = char
                if width == 2:
                    self.cells[self.row][self.column + 1] = ''
                self.column += width

    def csi(self, command, params):
        values = [int(value) if value else 0 for value in params.split(';')] if params else []
        if command == 'A':
            self.row = max(0, self.row - (values[0] if values and values[0] else 1))
        elif command == 'B':
            self.row = min(self.rows - 1, self.row + (values[0] if values and values[0] else 1))
        elif command == 'K':
            self.cells[self.row] = [' '] * self.columns
        elif command == 'L':
            count = values[0] if values and values[0] else 1
            for _ in range(count):
                self.cells.insert(self.row, [' '] * self.columns)
                self.cells.pop()
        elif command == 'M':
            count = values[0] if values and values[0] else 1
            for _ in range(count):
                self.cells.pop(self.row)
                self.cells.append([' '] * self.columns)
        elif command == 'J' and values and values[0] == 2:
            self.cells = [[' '] * self.columns for _ in range(self.rows)]

    def feed(self, data):
        data = self.pending + data
        self.pending = b''
        index = 0
        plain = bytearray()
        while index < len(data):
            if data[index:index + 2] == b'\x1b[':
                if plain:
                    self.write(bytes(plain))
                    plain.clear()
                end = index + 2
                while end < len(data) and not (0x40 <= data[end] <= 0x7e):
                    end += 1
                if end == len(data):
                    self.pending = data[index:]
                    break
                params = data[index + 2:end].decode('ascii', errors='ignore')
                self.csi(chr(data[end]), params)
                index = end + 1
            elif data[index] == 0x1b:
                if index + 1 == len(data):
                    self.pending = data[index:]
                    break
                index += 2
            else:
                plain.append(data[index])
                index += 1
        if plain:
            self.write(bytes(plain))

    def line(self, row):
        return ''.join(self.cells[row]).rstrip()

    def has(self, text):
        return any(self.line(row).startswith(text) for row in range(self.rows))

    def find(self, text):
        for row in range(self.rows):
            if self.line(row).startswith(text):
                return row
        return None


def capture(binary, args, rows, columns, prompt_row, observe=None):
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack('HHHH', rows, columns, 0, 0))
    env = os.environ.copy()
    env['COLUMNS'] = str(columns)
    env['LINES'] = str(rows)
    process = subprocess.Popen([binary] + args, stdin=slave, stdout=slave, stderr=slave, env=env)
    os.close(slave)
    screen = Screen(rows, columns)
    for _ in range(prompt_row):
        screen.newline()
    screen.write(b'[shell]$ rb --db sync')
    screen.newline()
    output = bytearray()
    progress_started = False
    deadline = time.monotonic() + 20
    try:
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0 or not select.select([master], [], [], remaining)[0]:
                raise AssertionError('progress probe timed out')
            try:
                chunk = os.read(master, 4096)
            except OSError:
                break
            if not chunk:
                break
            output.extend(chunk)
            if observe is None:
                screen.feed(chunk)
            else:
                for line in chunk.splitlines(keepends=True):
                    screen.feed(line)
                    observe(screen)
            if not screen.has('[shell]$ rb --db sync'):
                raise AssertionError('prompt was erased or scrolled during rendering')
            if not progress_started and b'resolved' in output:
                prompt = screen.find('[shell]$ rb --db sync')
                if not screen.line(prompt + 1).strip():
                    raise AssertionError('renderer left an empty row below the prompt')
                progress_started = True
        if process.wait(timeout=5) != 0:
            raise AssertionError('progress probe exited unsuccessfully')
        return screen, output
    except Exception as error:
        raise AssertionError(f'{args} at {columns}x{rows}: {error}\n' + '\n'.join(screen.line(row) for row in range(rows))) from error
    finally:
        if process.poll() is None:
            process.kill()
        process.wait(timeout=5)
        os.close(master)


def run(binary, tasks=False, handoff_output=False, stress=False, failure=False, rows=24, columns=80, prompt_row=8):
    args = ['--failure'] if failure else ['--stress'] if stress else ['--tasks'] if tasks else ['--handoff-output'] if handoff_output else []
    screen, output = capture(binary, args, rows, columns, prompt_row)
    if not screen.has('[shell]$ rb --db sync'):
        raise SystemExit(f'prompt was erased or scrolled\n{screen.line(1)!r}')
    if b'\x1b[2J' in output:
        raise SystemExit('renderer cleared the entire terminal')
    if b'probe complete' not in output:
        raise SystemExit('probe did not complete')
    if tasks or stress or failure:
        title = screen.find('┌─ × processing documents' if failure else '┌─ ✓ processing documents')
        summary = screen.find('└─ × processing failed' if failure else '└─ ✓ documents ready')
        prompt = screen.find('[shell]$ rb --db sync')
        if title != prompt + 1 or summary != title + (2 if failure else 1):
            raise SystemExit('task-only reporter left duplicate or blank rows')
        if failure and not any('document failure' in screen.line(row) for row in range(rows)):
            raise SystemExit('final report lost the task error')
        if screen.find('probe complete') != summary + 1:
            raise SystemExit('task-only final output did not start on the next line')
        titles = sum('processing documents' in screen.line(row) for row in range(screen.rows))
        if titles != 1 or any(screen.line(row) for row in range(summary + 2, screen.rows)):
            raise SystemExit('task-only completion left duplicated or stale rows')
        required = [] if stress or failure else [b'preparing content', b'exporting documents', b'reading document', b'writing document']
        for text in required:
            if text not in output:
                raise SystemExit(f'task-only reporter lost {text!r}')
        if stress and columns >= 80 and '資料'.encode() not in output:
            raise SystemExit('stress probe lost Unicode output')
        print(f'PASS PTY generic tasks {columns}x{rows} prompt={prompt_row} failure={failure}')
        return
    completed_row = screen.find('├─ ✓ preparing pty-probe bundle')
    if completed_row is None:
        raise SystemExit('final bundle row was not completed')
    if sum(line.startswith('├─ ✓ preparing pty-probe bundle') for line in (screen.line(row) for row in range(screen.rows))) != 1:
        raise SystemExit('final bundle row was duplicated')
    if screen.has('├─ ● preparing pty-probe bundle'):
        raise SystemExit('final bundle row remained active')
    if screen.find('├─ ✓ preparing native extensions') is None:
        raise SystemExit('native handoff row was lost')
    resolved_row = screen.find('┌─ ● preparing pty-probe bundle')
    if resolved_row is None:
        raise SystemExit('top tree entry was not preserved')
    summary_row = screen.find('└─ ✓ your meticulously prepared bundle is ready, sir')
    if summary_row != completed_row + 1:
        raise SystemExit('final bundle summary was not placed below the tree')
    probe_row = screen.find('probe complete')
    if probe_row != summary_row + 1:
        raise SystemExit('final output was not placed directly below the completed tree')
    if handoff_output and any('● preparing native extensions' in screen.line(row) or 'preview ' in screen.line(row) for row in range(screen.rows)):
        raise SystemExit('handoff left active rows or worker previews in the completed tree')
    print('PASS PTY prompt and viewport preservation')


def verify_aggregate_completion(binary):
    for rows in [4, 8]:
        screen, output = capture(binary, ['--aggregate'], rows, 40, 0)
        assert screen.find('[shell]$ rb --db sync') == 0
        assert screen.find('┌─ ✓ processing documents') == 1
        summary = screen.find('└─ ✓ documents ready')
        assert summary is not None and summary < rows - 1
        assert screen.row == summary + 1 and screen.column == 0
        assert not any(screen.line(row) for row in range(summary + 1, rows))
        assert b'\x1b[2J' not in output
        print(f'PASS PTY aggregate completion 40x{rows}')


def verify_standalone_parallel(binary):
    plain = subprocess.run([binary], capture_output=True, timeout=20)
    assert plain.returncode == 0
    assert b'completed 3 | failed 0' in plain.stderr
    assert b'\x1b' not in plain.stderr and b'\r' not in plain.stderr
    for worker in range(3):
        assert f'[worker: {worker}] complete'.encode() in plain.stderr
    for rows, columns in [(24, 80), (8, 40), (4, 40)]:
        simultaneous = []
        def observe(screen):
            frame = '\n'.join(screen.line(row) for row in range(rows))
            if all(f'worker {worker}' in frame for worker in range(3)):
                simultaneous.append(frame)
        screen, output = capture(binary, [], rows, columns, 0, observe)
        if rows == 24:
            assert simultaneous, 'parallel workers were never visible together'
        assert screen.find('[shell]$ rb --db sync') == 0
        assert screen.find('┌─ ✓ Preparing the dining room') == 1
        summary = screen.find('└─ ✓ The dining room is ready, sir')
        assert summary is not None
        assert screen.row == summary + 1 and screen.column == 0
        assert all(screen.line(row) for row in range(summary + 1))
        assert not any(screen.line(row) for row in range(summary + 1, rows))
        assert sum('Preparing the dining room' in screen.line(row) for row in range(rows)) == 1
        assert b'\x1b[2J' not in output
        print(f'PASS PTY standalone parallel {columns}x{rows}')


def verify_parser():
    data = 'prompt\r\n資料 e\u0301\x1b[1A\r\x1b[2Ktitle'.encode()
    expected = Screen(8, 40)
    expected.feed(data)
    assert expected.line(0) == 'title'
    assert expected.line(1) == '資料 e\u0301'
    assert expected.cells[1][2] == '料'
    assert expected.cells[1][5] == 'e\u0301'
    for size in [1, 2, 3, 5, 7]:
        fragmented = Screen(8, 40)
        for start in range(0, len(data), size):
            fragmented.feed(data[start:start + size])
        assert fragmented.cells == expected.cells, f'parser lost bytes at chunk size {size}'
        assert (fragmented.row, fragmented.column) == (expected.row, expected.column)


if __name__ == '__main__':
    verify_parser()
    if len(sys.argv) != 3:
        raise SystemExit('usage: pty_ui_test.py PATH_TO_PROBE PATH_TO_PARALLEL')
    plain = subprocess.run([sys.argv[1], '--failure'], capture_output=True, timeout=20)
    assert plain.returncode == 0
    assert b'[worker: ' in plain.stderr and b'failed: document failure' in plain.stderr
    assert b'\x1b' not in plain.stderr and b'\r' not in plain.stderr
    print('PASS redirected output uses worker-prefixed plain reporting')
    verify_standalone_parallel(sys.argv[2])
    verify_aggregate_completion(sys.argv[1])
    run(sys.argv[1])
    run(sys.argv[1], tasks=True)
    run(sys.argv[1], handoff_output=True)
    for rows in [8, 12, 24, 40]:
        for columns in [40, 80, 120]:
            for prompt in [0, rows // 2]:
                run(sys.argv[1], stress=True, rows=rows, columns=columns, prompt_row=prompt)
    run(sys.argv[1], failure=True)
    run(sys.argv[1], failure=True, rows=8, columns=40, prompt_row=0)
