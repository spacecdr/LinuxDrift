use flux::{
    settings::{ColorMode, ColorPreset, PressureMode},
    Settings,
};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub fn path() -> Result<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|v| Path::new(v).is_absolute()) {
        return Ok(PathBuf::from(dir).join("linuxdrift/config.json"));
    }
    Ok(
        PathBuf::from(std::env::var_os("HOME").ok_or("HOME is not set")?)
            .join(".config/linuxdrift/config.json"),
    )
}

// One schema supplies both validation and the graphical controls.
pub fn ranges() -> Value {
    json!({
        "fluidSize": [16, 512, 16], "fluidFrameRate": [1, 240, 1],
        "fluidTimestep": [0.001, 0.1, 0.001], "viscosity": [0.1, 8, 0.1],
        "velocityDissipation": [0, 2, 0.1], "diffusionIterations": [0, 30, 1],
        "pressureIterations": [0, 60, 1], "lineLength": [1, 1000, 1],
        "lineWidth": [0.1, 100, 0.1], "lineBeginOffset": [0, 1, 0.01],
        "lineVariance": [0, 1, 0.01], "gridSpacing": [1, 100, 1],
        "viewScale": [0.1, 10, 0.1], "overallScale": [0.1, 10, 0.1],
        "noiseMultiplier": [0, 10, 0.01]
    })
}

pub fn validate(settings: &Settings) -> Result<()> {
    let value = serde_json::to_value(settings)?;
    for (key, bounds) in ranges().as_object().unwrap() {
        let number = value[key]
            .as_f64()
            .ok_or_else(|| format!("Invalid {key}"))?;
        if !number.is_finite()
            || number < bounds[0].as_f64().unwrap()
            || number > bounds[1].as_f64().unwrap()
        {
            return Err(format!("{key} must be between {} and {}", bounds[0], bounds[1]).into());
        }
    }
    if settings.noise_channels.is_empty() || settings.noise_channels.len() > 16 {
        return Err("Provide between 1 and 16 noise channels".into());
    }
    for channel in &settings.noise_channels {
        if !(0.01..=1000.0).contains(&channel.scale)
            || !(0.0..=10.0).contains(&channel.multiplier)
            || !(0.0..=1.0).contains(&channel.offset_increment)
        {
            return Err(
                "Invalid noise channel (scale 0.01–1000, multiplier 0–10, offsetIncrement 0–1)"
                    .into(),
            );
        }
    }
    if let PressureMode::ClearWith(value) = settings.pressure_mode {
        if !value.is_finite() || value.abs() > 100.0 {
            return Err("Invalid pressure reset value".into());
        }
    }
    if let ColorMode::ImageFile(path) = &settings.color_mode {
        if !path.to_string_lossy().starts_with("builtin:") && !path.is_absolute() {
            return Err("The palette image must use an absolute path".into());
        }
    }
    Ok(())
}

pub fn parse(bytes: &[u8]) -> Result<Settings> {
    let value: Value = serde_json::from_slice(bytes)?;
    let defaults = serde_json::to_value(Settings::default())?;
    for key in value
        .as_object()
        .ok_or("Configuration must be a JSON object")?
        .keys()
    {
        if defaults.get(key).is_none() {
            return Err(format!("Unknown setting: {key}").into());
        }
    }
    let settings: Settings = serde_json::from_value(value)?;
    validate(&settings)?;
    Ok(settings)
}

pub fn load(path: &Path) -> Result<Settings> {
    match std::fs::File::open(path) {
        Ok(file) => {
            let mut bytes = Vec::new();
            file.take(1024 * 1024).read_to_end(&mut bytes)?;
            parse(&bytes).map_err(|e| format!("{}: {e}", path.display()).into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
        Err(e) => Err(e.into()),
    }
}

pub fn save(path: &Path, settings: &Settings) -> Result<()> {
    validate(settings)?;
    let parent = path.parent().ok_or("Invalid configuration path")?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    serde_json::to_writer_pretty(&mut file, settings)?;
    file.write_all(b"\n")?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

pub fn palette(settings: &Settings) -> Result<Option<image::RgbaImage>> {
    let bytes = match &settings.color_mode {
        ColorMode::ImageFile(path) => match path.to_str() {
            Some("builtin:gumdrop") => include_bytes!("../assets/gumdrop.png").to_vec(),
            Some("builtin:silver") => include_bytes!("../assets/silver.png").to_vec(),
            Some("builtin:charcoal") => include_bytes!("../assets/charcoal.png").to_vec(),
            Some("builtin:glitter") => include_bytes!("../assets/glitter.png").to_vec(),
            Some("builtin:galaxy") => include_bytes!("../assets/galaxy.png").to_vec(),
            Some("builtin:verdant") => include_bytes!("../assets/verdant.png").to_vec(),
            Some("builtin:freedom") => include_bytes!("../assets/freedom.png").to_vec(),
            _ => std::fs::read(path).map_err(|e| format!("Palette {}: {e}", path.display()))?,
        },
        ColorMode::Preset(ColorPreset::Freedom) => include_bytes!("../assets/freedom.png").to_vec(),
        _ => return Ok(None),
    };
    Ok(Some(
        flux::render::color::Context::decode_color_texture(&bytes)
            .map_err(|e| format!("Cannot decode palette: {e}"))?,
    ))
}

pub fn edit(path: &Path, settings: Settings) -> Result<()> {
    let executable = std::env::current_exe()?;
    let mut child = Command::new("/usr/bin/python3")
        .args(["-c", include_str!("config_ui.py")])
        .arg(executable)
        .arg(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Configuration requires Python 3, PyGObject and GTK 4: {e}"))?;
    serde_json::to_writer(
        child.stdin.take().unwrap(),
        &json!({"settings": settings, "defaults": Settings::default(), "ranges": ranges()}),
    )?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err("Configuration window failed; install python3-gi and gir1.2-gtk-4.0".into());
    }
    // Cancel/close returns no payload. Saving is validated and atomic in Rust.
    if !output.stdout.is_empty() {
        let settings = parse(&output.stdout)?;
        palette(&settings)?;
        save(path, &settings)?;
        println!("Saved {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn round_trip_and_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("linuxdrift/config.json");
        assert_eq!(load(&path).unwrap().line_length, 450.0);
        let mut settings = Settings::default();
        settings.line_length = 321.0;
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap().line_length, 321.0);
        settings.line_length = 123.0;
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap().line_length, 123.0);
    }
    #[test]
    fn rejects_bad_config_without_overwriting() {
        for json in [
            r#"{"gridSpacing":0}"#,
            r#"{"fluidFrameRate":0}"#,
            r#"{"typo":1}"#,
            r#"{"noiseChannels":[]}"#,
            "[]",
            "{",
        ] {
            assert!(parse(json.as_bytes()).is_err(), "{json}");
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save(&path, &Settings::default()).unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut settings = Settings::default();
        settings.view_scale = 0.0;
        assert!(save(&path, &settings).is_err());
        assert_eq!(before, std::fs::read(&path).unwrap());
    }
    #[test]
    fn all_embedded_palettes_decode() {
        for name in [
            "gumdrop", "silver", "freedom", "charcoal", "glitter", "galaxy", "verdant",
        ] {
            let mut settings = Settings::default();
            settings.color_mode = ColorMode::ImageFile(format!("builtin:{name}").into());
            assert!(palette(&settings).unwrap().is_some());
        }
    }
}
