"""GTK settings front end. Rust owns validation and persistent writes."""
import copy
import json
import os
import subprocess
import sys
import tempfile

import gi
gi.require_version("Gtk", "4.0")
from gi.repository import Gtk, Gio


def run():
    payload = json.load(sys.stdin)
    executable, config_path = sys.argv[1:3]
    app = Gtk.Application(application_id="io.github.linuxdrift.Config", flags=Gio.ApplicationFlags.NON_UNIQUE)
    preview = None
    temporary = None

    def activate(app):
        nonlocal preview, temporary
        window = Gtk.ApplicationWindow(application=app, title="LinuxDrift — Impostazioni")
        window.set_default_size(680, 780)
        outer = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        for edge in ("top", "bottom", "start", "end"):
            getattr(outer, "set_margin_" + edge)(18)
        window.set_child(outer)
        title = Gtk.Label(label="LinuxDrift", xalign=0)
        title.add_css_class("title-1")
        outer.append(title)
        location = Gtk.Label(label=config_path, xalign=0, selectable=True, wrap=True)
        location.add_css_class("dim-label")
        outer.append(location)
        scroll = Gtk.ScrolledWindow(vexpand=True, hscrollbar_policy=Gtk.PolicyType.NEVER)
        outer.append(scroll)
        grid = Gtk.Grid(column_spacing=24, row_spacing=10)
        scroll.set_child(grid)
        getters = {}
        row = 0

        def add(label, widget):
            nonlocal row
            grid.attach(Gtk.Label(label=label, xalign=0), 0, row, 1, 1)
            widget.set_hexpand(True)
            grid.attach(widget, 1, row, 1, 1)
            row += 1
            return widget

        def combo(label, values, selected):
            widget = Gtk.ComboBoxText()
            for value in values:
                widget.append(value, value)
            widget.set_active_id(selected)
            return add(label, widget)

        def spin(label, value, bounds, integer=False):
            low, high, step = bounds
            widget = Gtk.SpinButton.new_with_range(low, high, step)
            widget.set_digits(0 if integer else 4)
            widget.set_value(value)
            add(label, widget)
            return widget.get_value_as_int if integer else widget.get_value

        settings = copy.deepcopy(payload["settings"])
        modes = ["Normal", "DebugNoise", "DebugFluid", "DebugPressure", "DebugDivergence"]
        mode = combo("Visualizzazione", modes, settings["mode"])
        getters["mode"] = mode.get_active_id
        seed = add("Seed (vuoto = casuale)", Gtk.Entry(text=settings["seed"] or ""))
        getters["seed"] = lambda: seed.get_text() or None
        color = settings["colorMode"]
        selected = color.get("Preset") or color.get("ImageFile", "").removeprefix("builtin:").title()
        if selected == "Galaxy":
            selected = "Andromeda"
        palettes = ["Original", "Plasma", "Poolside", "Gumdrop", "Silver", "Charcoal", "Glitter", "Andromeda", "Verdant", "Freedom", "Immagine"]
        if selected not in palettes:
            selected = "Immagine"
        colors = combo("Palette", palettes, selected)
        image_path = add("Immagine (percorso assoluto)", Gtk.Entry(text=color.get("ImageFile", "") if selected == "Immagine" else ""))

        def get_color():
            name = colors.get_active_id()
            if name == "Immagine":
                return {"ImageFile": image_path.get_text()}
            if name == "Andromeda":
                return {"ImageFile": "builtin:galaxy"}
            if name in ("Gumdrop", "Silver", "Charcoal", "Glitter", "Verdant", "Freedom"):
                return {"ImageFile": "builtin:" + name.lower()}
            return {"Preset": name}
        getters["colorMode"] = get_color
        labels = {"fluidSize": "Risoluzione fluido", "fluidFrameRate": "Passi fluido al secondo", "fluidTimestep": "Intervallo simulazione", "viscosity": "Viscosità", "velocityDissipation": "Dissipazione velocità", "diffusionIterations": "Iterazioni diffusione", "pressureIterations": "Iterazioni pressione", "lineLength": "Lunghezza linee", "lineWidth": "Larghezza linee", "lineBeginOffset": "Inizio sfumatura", "lineVariance": "Varianza", "gridSpacing": "Distanza griglia", "viewScale": "Zoom", "overallScale": "Scala complessiva (1 = 100%)", "noiseMultiplier": "Intensità rumore"}
        for key, bounds in payload["ranges"].items():
            getters[key] = spin(labels[key], settings[key], bounds, isinstance(payload["defaults"][key], int))
        pressure = settings["pressureMode"]
        pressure_mode = combo("Pressione", ["ClearWith", "Retain"], "Retain" if pressure == "Retain" else "ClearWith")
        pressure_value = spin("Valore reset pressione", pressure.get("ClearWith", 0) if isinstance(pressure, dict) else 0, [-100, 100, 0.1])
        getters["pressureMode"] = lambda: "Retain" if pressure_mode.get_active_id() == "Retain" else {"ClearWith": pressure_value()}
        channels = []
        for index, channel in enumerate(settings["noiseChannels"], 1):
            channels.append({key: spin(f"Rumore {index} — {label}", channel[key], bounds) for key, label, bounds in [("scale", "scala", [0.01, 1000, 0.01]), ("multiplier", "intensità", [0, 10, 0.01]), ("offsetIncrement", "velocità", [0, 1, 0.0001])]})
        getters["noiseChannels"] = lambda: [{key: get() for key, get in channel.items()} for channel in channels]
        error = Gtk.Label(xalign=0, wrap=True)
        error.add_css_class("error")
        outer.append(error)
        buttons = Gtk.Box(spacing=8)
        outer.append(buttons)

        def values():
            result = {key: get() for key, get in getters.items()}
            # Validate through the same code as the screensaver, before closing.
            with tempfile.TemporaryDirectory(prefix="linuxdrift-check-") as directory:
                env = dict(os.environ, XDG_CONFIG_HOME=directory)
                os.mkdir(os.path.join(directory, "linuxdrift"))
                with open(os.path.join(directory, "linuxdrift/config.json"), "w") as file:
                    json.dump(result, file)
                check = subprocess.run([executable, "--check-config"], env=env, capture_output=True, text=True)
                if check.returncode:
                    raise ValueError(check.stderr.strip())
            return result

        def stop_preview():
            nonlocal preview, temporary
            if preview is not None:
                if preview.poll() is None:
                    preview.terminate()
                    preview.wait(timeout=5)
                preview = None
            if temporary is not None:
                temporary.cleanup()
                temporary = None

        def save(_):
            try:
                result = values()
                stop_preview()
                print(json.dumps(result), flush=True)
                app.quit()
            except (ValueError, OSError) as exc:
                error.set_text(str(exc))

        def show_preview(_):
            nonlocal preview, temporary
            try:
                result = values()
                stop_preview()
                temporary = tempfile.TemporaryDirectory(prefix="linuxdrift-preview-")
                os.mkdir(os.path.join(temporary.name, "linuxdrift"))
                with open(os.path.join(temporary.name, "linuxdrift/config.json"), "w") as file:
                    json.dump(result, file)
                preview = subprocess.Popen([executable, "--preview"], env=dict(os.environ, XDG_CONFIG_HOME=temporary.name), stdout=sys.stderr)
                error.set_text("")
            except (ValueError, OSError) as exc:
                error.set_text(str(exc))

        def reset(_):
            stop_preview()
            payload["settings"] = copy.deepcopy(payload["defaults"])
            activate(app)
            window.destroy()

        for label, callback in [("Ripristina", reset), ("Anteprima", show_preview), ("Annulla", lambda _: app.quit()), ("Salva", save)]:
            button = Gtk.Button(label=label)
            if label == "Salva":
                button.add_css_class("suggested-action")
            button.connect("clicked", callback)
            buttons.append(button)
        app.connect("shutdown", lambda _: stop_preview())
        window.present()

    app.connect("activate", activate)
    app.run([])


if __name__ == "__main__":
    run()
