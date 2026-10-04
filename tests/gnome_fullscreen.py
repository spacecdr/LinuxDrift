#!/usr/bin/python3
"""Regression: Mutter must not restore a fullscreen saver to 1280x800.

Uses an isolated GNOME Shell and a 1920x1080 virtual monitor; never replaces
or changes the user's running desktop. Requires gnome-shell and dbus-run-session.
LINUXDRIFT_TEST_BINARY can point to an older binary to reproduce the bug.
"""
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import time

BINARY = os.environ.get('LINUXDRIFT_TEST_BINARY', str(Path(__file__).resolve().parents[1] / 'target/release/linuxdrift'))


def check_window(env, arguments, expected, fullscreen):
    output = subprocess.run(
        [BINARY, *arguments, '--duration', '6', '--set', 'fluidSize=32', '--set', 'pressureIterations=8'],
        env=dict(env, WAYLAND_DEBUG='1', RUST_LOG='linuxdrift=info'),
        capture_output=True, text=True, timeout=45,
    )
    label = 'fullscreen' if fullscreen else 'preview'
    Path('/tmp/linuxdrift-gnome-' + label + '.log').write_text(output.stderr)
    assert output.returncode == 0, output.stderr[-4000:]
    assert 'Rendered ' in output.stderr, output.stderr[-4000:]
    sizes = [tuple(map(int, pair)) for pair in re.findall(
        r'xdg_toplevel@\d+\.configure\((\d+), (\d+),', output.stderr
    ) if pair != ('0', '0')]
    # Floating windows may receive (0, 0): the compositor lets the client
    # choose its preferred size. Fullscreen must have an explicit output size.
    if fullscreen or sizes:
        assert sizes and sizes[-1] == expected, f'{label}: final compositor size {sizes}, expected {expected}'
    expected_state = f'Window configured: {expected[0]}x{expected[1]}, fullscreen={str(fullscreen).lower()}'
    assert expected_state in output.stderr, f'Missing compositor state: {expected_state}'
    if fullscreen:
        # Catch the actual cause, not just an initial fullscreen configure.
        limits = re.findall(r'xdg_toplevel@\d+\.set_max_size\((\d+), (\d+)\)', output.stderr)
        assert all(pair == ('0', '0') for pair in limits), f'Unexpected fixed-size limits: {limits}'
        states = re.findall(r'Window configured: .*fullscreen=(true|false)', output.stderr)
        assert states[-1] == 'true', states
    print(f'GNOME {label}: {expected[0]}x{expected[1]}, persistent fullscreen={fullscreen}: PASS', flush=True)


def main():
    with tempfile.TemporaryDirectory(prefix='linuxdrift-gnome-') as directory:
        os.chmod(directory, 0o700)
        env = dict(os.environ, XDG_RUNTIME_DIR=directory, HOME=directory,
                   XDG_CONFIG_HOME=directory, GSETTINGS_BACKEND='memory', LIBGL_ALWAYS_SOFTWARE='1',
                   XDG_SESSION_TYPE='wayland', XDG_CURRENT_DESKTOP='GNOME',
                   GNOME_SHELL_SESSION_MODE='gnome', GIO_USE_VFS='local', GTK_USE_PORTAL='0', WAYLAND_DISPLAY='linuxdrift-gnome',
                   VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
        env.pop('XSCREENSAVER_WINDOW', None)
        with open('/tmp/linuxdrift-gnome-shell.log', 'w') as log:
            shell = subprocess.Popen(
                ['dbus-run-session', '--', 'gnome-shell', '--headless', '--wayland', '--no-x11',
                 '--virtual-monitor', '1920x1080', '--wayland-display', 'linuxdrift-gnome'],
                env=env, stdout=log, stderr=log, start_new_session=True,
            )
            try:
                for _ in range(200):
                    if Path(directory, 'linuxdrift-gnome').exists():
                        break
                    if shell.poll() is not None:
                        raise RuntimeError(Path('/tmp/linuxdrift-gnome-shell.log').read_text()[-4000:])
                    time.sleep(0.1)
                else:
                    raise RuntimeError('GNOME did not create its Wayland socket')
                check_window(env, [], (1920, 1080), True)
                check_window(env, ['--preview'], (1280, 800), False)
            finally:
                if shell.poll() is None:
                    os.killpg(shell.pid, signal.SIGTERM)
                    try:
                        shell.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(shell.pid, signal.SIGKILL)
                        shell.wait()
                # Portal services may briefly leave disconnected FUSE mounts
                # in this test's private runtime directory after D-Bus exits.
                for mount in ('doc', 'gvfs'):
                    subprocess.run(['fusermount3', '-u', '-z', str(Path(directory, mount))],
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


if __name__ == '__main__':
    main()
