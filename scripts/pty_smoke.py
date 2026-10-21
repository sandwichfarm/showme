#!/usr/bin/env python3
"""Exercise the real CLI through a Unix pseudo-terminal with bounded waits."""

import argparse
import fcntl
import os
from pathlib import Path
import pty
import select
import signal
import struct
import subprocess
import termios
import time
import tty
import unittest

BINARY = None
FIXTURE = Path(__file__).resolve().parent.parent / "tests/fixtures/red.ppm"


def comparable_mode(attributes):
    attributes = list(attributes)
    # Darwin may set the kernel's pending-input bookkeeping bit after tcsetattr.
    # Compare every actual input/output/raw-mode setting without that transient bit.
    attributes[3] &= ~getattr(termios, "PENDIN", 0)
    return attributes


class Terminal:
    def __init__(self, arguments, piped=None, raw=False, keep_input_open=False):
        self.master, self.slave = pty.openpty()
        os.set_blocking(self.master, False)
        fcntl.ioctl(self.slave, termios.TIOCSWINSZ, struct.pack("HHHH", 24, 80, 0, 0))
        if raw:
            tty.setraw(self.slave)
        self.before = comparable_mode(termios.tcgetattr(self.slave))
        environment = os.environ.copy()
        for key in ("TMUX", "STY", "KITTY_WINDOW_ID", "TERM_PROGRAM"):
            environment.pop(key, None)
        environment["TERM"] = "xterm-256color"

        self.process = subprocess.Popen([str(BINARY), "--backend", "unicode", *map(str, arguments)],
                                        stdin=subprocess.PIPE if piped is not None else self.slave,
                                        stdout=self.slave, stderr=subprocess.PIPE, env=environment,
                                        start_new_session=True)
        if piped is not None:
            self.process.stdin.write(piped)
            if not keep_input_open:
                self.process.stdin.close()
        self.output = bytearray()

    def run(self, respond=False, interrupt=False, interrupt_marker=b"\x1b[0m"):
        started = time.monotonic()
        answered = 0
        interrupted_at = None
        try:
            while self.process.poll() is None:
                if time.monotonic() - started > 5:
                    raise TimeoutError("CLI exceeded PTY deadline")
                if select.select([self.master], [], [], 0.02)[0]:
                    try:
                        self.output.extend(os.read(self.master, 65536))
                    except BlockingIOError:
                        pass
                queries = self.output.count(b"\x1b[16t")
                if respond:
                    for _ in range(queries - answered):
                        os.write(self.master, b"\x1b[6;16;8t")
                    answered = queries
                if interrupt and interrupted_at is None and interrupt_marker in self.output:
                    interrupted_at = time.monotonic()
                    self.process.send_signal(signal.SIGINT)
            while select.select([self.master], [], [], 0)[0]:
                try:
                    chunk = os.read(self.master, 65536)
                except BlockingIOError:
                    break
                if not chunk:
                    break
                self.output.extend(chunk)
            restored = comparable_mode(termios.tcgetattr(self.slave))
            error = self.process.stderr.read()
            interrupt_delay = None if interrupted_at is None else time.monotonic() - interrupted_at
            return self.process.returncode, bytes(self.output), error, restored, interrupt_delay
        finally:
            if self.process.poll() is None:
                self.process.kill()
                self.process.wait()
            if self.process.stdin is not None and not self.process.stdin.closed:
                self.process.stdin.close()
            self.process.stderr.close()
            os.close(self.master)
            os.close(self.slave)


class LifecycleTests(unittest.TestCase):
    def test_query_timeout_restores_original_terminal_mode(self):
        for raw in (False, True):
            with self.subTest(raw=raw):
                terminal = Terminal(["--quiet", FIXTURE], raw=raw)
                code, output, error, restored, _ = terminal.run()
                self.assertEqual(code, 0, error)
                self.assertIn(b"\x1b[16t", output)
                self.assertEqual(restored, terminal.before)

    def test_cell_size_reply_becomes_terminal_pixel_dimensions(self):
        terminal = Terminal(["--verbose", FIXTURE])
        code, _, error, restored, _ = terminal.run(respond=True)
        self.assertEqual(code, 0, error)
        self.assertIn(b"Pixel size: 640x384", error)
        self.assertEqual(restored, terminal.before)

    def test_piped_image_bytes_are_preserved_without_terminal_queries(self):
        terminal = Terminal(["--quiet", "-"], piped=FIXTURE.read_bytes())
        code, output, error, restored, _ = terminal.run()
        self.assertEqual(code, 0, error)
        self.assertNotIn(b"\x1b[16t", output)
        self.assertIn(b"\x1b[38;", output)
        self.assertEqual(restored, terminal.before)

    def test_ordinary_error_restores_cursor_and_alternate_screen(self):
        terminal = Terminal(["--quiet", "--alternate-screen", FIXTURE.with_name("missing.ppm")])
        code, output, _, restored, _ = terminal.run()
        self.assertEqual(code, 1)
        self.assertIn(b"\x1b[?1049h", output)
        self.assertIn(b"\x1b[?25l", output)
        self.assertTrue(output.endswith(b"\x1b[?25h\x1b[?1049l"))
        self.assertEqual(restored, terminal.before)

    def test_interrupt_cancels_long_wait_and_restores_terminal(self):
        terminal = Terminal(["--quiet", "--alternate-screen", "--wait", "30s", FIXTURE, FIXTURE])
        code, output, error, restored, delay = terminal.run(interrupt=True)
        self.assertEqual(code, 130, error)
        self.assertTrue(output.endswith(b"\x1b[?25h\x1b[?1049l"))
        self.assertEqual(restored, terminal.before)
        self.assertIsNotNone(delay)
        self.assertLess(delay, 1.0)

    def test_interrupt_cancels_a_pending_stdin_read(self):
        terminal = Terminal(["--quiet", "--alternate-screen", "-"], piped=b"", keep_input_open=True)
        code, output, error, restored, delay = terminal.run(interrupt=True, interrupt_marker=b"\x1b[?25l")
        self.assertEqual(code, 130, error)
        self.assertTrue(output.endswith(b"\x1b[?25h\x1b[?1049l"))
        self.assertEqual(restored, terminal.before)
        self.assertIsNotNone(delay)
        self.assertLess(delay, 1.0)

    def test_broken_output_pipe_returns_an_error(self):
        read_end, write_end = os.pipe()
        os.close(read_end)
        try:
            process = subprocess.Popen([str(BINARY), "--quiet", "-o", "/dev/stdout", str(FIXTURE)],
                                       stdin=subprocess.DEVNULL, stdout=write_end, stderr=subprocess.PIPE)
        finally:
            os.close(write_end)
        _, error = process.communicate(timeout=5)
        self.assertEqual(process.returncode, 1, error)
        self.assertIn(b"broken pipe", error.lower())


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    args = parser.parse_args()
    BINARY = args.binary.resolve()
    unittest.main(argv=[__file__], verbosity=2)
