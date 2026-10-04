#!/usr/bin/python3
"""Run under Xvfb. Exercise real GTK controls; Rust validates every result."""
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import traceback
from unittest.mock import patch
import gi
gi.require_version('Gtk', '4.0')
from gi.repository import Gtk, GLib

repo = Path(__file__).resolve().parents[1]
binary = str(repo / 'target/release/linuxdrift')
spec = importlib.util.spec_from_file_location('ui', repo / 'linuxdrift/src/config_ui.py')
ui = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ui)
with tempfile.TemporaryDirectory(prefix='linuxdrift-ui-defaults-') as defaults_dir:
    settings = json.loads(subprocess.check_output([binary, '--print-config'], env=dict(os.environ, XDG_CONFIG_HOME=defaults_dir)))
# Read the shared range schema through a tiny extraction of its JSON literal.
source = (repo / 'linuxdrift/src/config.rs').read_text()
start = source.index('json!({') + len('json!(')
end = source.index('})', start) + 1
schema = json.loads(source[start:end])
payload = dict(settings=settings, defaults=settings, ranges=schema, presets=json.loads((repo / "linuxdrift/src/presets.json").read_text()))
original_run = Gtk.Application.run
errors = []

def descendants(widget):
    yield widget
    child = widget.get_first_child()
    while child:
        yield from descendants(child)
        child = child.get_next_sibling()


def exercise(app, args):
    def action():
        try:
            window = app.get_active_window()
            widgets = list(descendants(window))
            spins = [w for w in widgets if isinstance(w, Gtk.SpinButton)]
            assert len(spins) == len(schema) + 4 + 3 * len(settings['noiseChannels'])
            combos = [w for w in widgets if isinstance(w, Gtk.ComboBoxText)]
            colors = next(w for w in combos if w.get_active_id() == 'Original')
            colors.set_active_id('Andromeda')
            preset = next(w for w in combos if w.get_name() == 'performance-preset')
            preset.set_active_id('haswell')
            enabled = next(w for w in widgets if w.get_name() == 'clock-enabled')
            enabled.set_active(True)
            position = next(w for w in combos if w.get_name() == 'clock-position')
            position.set_active_id('random')
            def control_for(text):
                label = next(w for w in widgets if isinstance(w, Gtk.Label) and w.get_text() == text)
                grid = label.get_parent()
                return grid.get_child_at(1, grid.query_child(label).row)
            control_for('Opacità sfondo (0 = nessuno)').set_value(0.65)
            control_for('Sfumatura verso il trasparente').set_active_id('100')
            control_for('Cambia posizione ogni (secondi)').set_value(15)
            control_for('Distanza griglia').set_value(28)
            assert preset.get_active_id() == 'custom'
            preset.set_active_id('haswell')
            button = next(w for w in widgets if isinstance(w, Gtk.Button) and w.get_label() == 'Salva')
            subprocess.run(['import', '-window', 'root', '/tmp/linuxdrift-config.png'], check=True)
            button.emit('clicked')
        except Exception:
            errors.append(traceback.format_exc())
            app.quit()
        return False
    GLib.timeout_add(2200, action)
    GLib.timeout_add_seconds(12, lambda: (errors.append('GTK test timed out'), app.quit(), False)[-1])
    return original_run(app, args)

with tempfile.TemporaryDirectory() as tmp, patch.object(Gtk.Application, 'run', exercise), patch.object(sys, 'argv', ['ui', binary, tmp + '/config.json']), patch.object(sys, 'stdin', io.StringIO(json.dumps(payload))), patch.object(sys, 'stdout', io.StringIO()) as output:
    ui.run()
    assert not errors, errors
    saved = json.loads(output.getvalue())
    assert saved['colorMode'] == {'ImageFile': 'builtin:galaxy'}
    assert saved['clock']['enabled'] and saved['clock']['position'] == 'random'
    assert saved['clock']['backgroundOpacity'] == 0.65
    assert saved['clock']['backgroundFeather'] == 100
    assert saved['clock']['moveInterval'] == 15
    assert saved['performancePreset'] == 'haswell'
    assert (saved['fluidSize'], saved['pressureIterations'], saved['gridSpacing']) == (64, 8, 24)
    assert saved['noiseChannels'] == settings['noiseChannels']
    assert set(saved) == set(settings)
print('GTK controls, palette selection and validated Save: PASS')
