#!/usr/bin/python3
"""Launch isolated X11 and Wayland servers and prove that frames render."""
import ctypes as C
import os
from pathlib import Path
import subprocess
import tempfile
import time

binary = str(Path(__file__).resolve().parents[1] / 'target/release/linuxdrift')

def run(env, args, name):
    output = subprocess.run([binary, *args, '--duration', '3'], env=env, capture_output=True, text=True, timeout=40)
    Path('/tmp/linuxdrift-' + name + '.log').write_text(output.stderr)
    assert output.returncode == 0, output.stderr
    assert 'Rendered ' in output.stderr, output.stderr
    if name == 'wayland-fullscreen':
        assert 'Window configured: 1920x1080, fullscreen=true' in output.stderr, output.stderr
    print(name + ': PASS (' + output.stderr.split('Rendered ')[-1].strip() + ')')

with tempfile.TemporaryDirectory(prefix='linuxdrift-test-') as directory:
    os.chmod(directory, 0o700)
    env = dict(os.environ, XDG_RUNTIME_DIR=directory, XDG_CONFIG_HOME=directory, RUST_LOG='linuxdrift=info', GSK_RENDERER='cairo', WGPU_BACKEND='vulkan', VK_ICD_FILENAMES=str(next(Path('/usr/share/vulkan/icd.d').glob('lvp_icd*.json'))))
    xvfb = subprocess.Popen(['Xvfb', '-displayfd', '1', '-screen', '0', '1280x800x24'], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
    env['DISPLAY'] = ':' + xvfb.stdout.readline().strip()
    env['XDG_SESSION_TYPE'] = 'x11'
    env.pop('WAYLAND_DISPLAY', None)
    try:
        run(env, [], 'x11-fullscreen')
        # Real parent window; child must match it and resize with it.
        os.environ['DISPLAY'] = env['DISPLAY']
        x = C.CDLL('libX11.so.6')
        x.XOpenDisplay.restype = C.c_void_p
        display = x.XOpenDisplay(None)
        assert display
        for name in ['XDefaultRootWindow', 'XCreateSimpleWindow']:
            getattr(x, name).restype = C.c_ulong
        x.XDefaultRootWindow.argtypes = [C.c_void_p]
        x.XCreateSimpleWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint, C.c_uint, C.c_ulong, C.c_ulong]
        x.XMapWindow.argtypes = [C.c_void_p, C.c_ulong]
        x.XResizeWindow.argtypes = [C.c_void_p, C.c_ulong, C.c_uint, C.c_uint]
        x.XFlush.argtypes = [C.c_void_p]
        parent = x.XCreateSimpleWindow(display, x.XDefaultRootWindow(display), 0, 0, 640, 480, 0, 0, 0)
        x.XMapWindow(display, parent); x.XFlush(display)
        child_env = dict(env, XSCREENSAVER_WINDOW=hex(parent), XDG_SESSION_TYPE='wayland')
        proc = subprocess.Popen([binary, '-root', '--duration', '22', '--palette', 'plasma', '--set', 'fluidSize=32', '--set', 'pressureIterations=8', '--set', 'clock={"enabled":true,"position":"random","moveInterval":5,"backgroundOpacity":0.65}'], env=child_env, stderr=subprocess.PIPE, text=True)
        time.sleep(3)
        x.XResizeWindow(display, parent, 800, 600); x.XFlush(display)
        time.sleep(13)
        tree = subprocess.check_output(['xwininfo', '-id', hex(parent), '-tree'], env=env, text=True)
        assert 'LinuxDrift' in tree and '800x600' in tree, tree
        subprocess.run(['import', '-window', str(parent), '/tmp/linuxdrift-render.png'], env=env, check=True)
        colors = int(subprocess.check_output(['identify', '-format', '%k', '/tmp/linuxdrift-render.png']))
        assert colors > 10, f'Rendered image is blank ({colors} colors)'
        _, stderr = proc.communicate(timeout=30)
        assert proc.returncode == 0 and 'Rendered ' in stderr, stderr
        assert stderr.count('Clock moved:') >= 2, stderr
        assert 'Clock texture refreshed:' in stderr, stderr
        Path('/tmp/linuxdrift-embedded.log').write_text(stderr)
        print('XScreenSaver parent embedding, resize and palette: PASS')
        subprocess.run(['xvfb-run', '-a', '/usr/bin/python3', str(Path(__file__).with_name('gui_smoke.py'))], env=env, check=True, timeout=30)
    finally:
        xvfb.terminate(); xvfb.wait()
    env['XDG_SESSION_TYPE'] = 'wayland'
    env['WAYLAND_DISPLAY'] = 'linuxdrift-test'
    log = open('/tmp/linuxdrift-weston.log', 'w')
    weston = subprocess.Popen(['weston', '--backend=headless-backend.so', '--socket=linuxdrift-test', '--idle-time=0', '--width=1920', '--height=1080'], env=env, stdout=log, stderr=log)
    try:
        for _ in range(100):
            if Path(directory, 'linuxdrift-test').exists(): break
            if weston.poll() is not None: raise RuntimeError(Path('/tmp/linuxdrift-weston.log').read_text())
            time.sleep(0.1)
        run(env, ['--clock', '--preset', 'eco'], 'wayland-fullscreen')
        assert 'Clock texture refreshed:' in Path('/tmp/linuxdrift-wayland-fullscreen.log').read_text()
    finally:
        weston.terminate(); weston.wait(); log.close()
