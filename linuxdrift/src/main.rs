mod clock;
mod config;
mod xscreensaver;
use clap::Parser;

use image::RgbaImage;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

#[cfg(target_os = "macos")]
use winit::platform::macos::WindowAttributesExtMacOS;

use flux::{display_size::DisplaySize, Flux, Settings};

struct App {
    flux: Flux,
    clock: Option<clock::ClockOverlay>,
}

struct GpuState {
    device: wgpu::Device,
    command_queue: wgpu::Queue,
    window_surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    size: Option<DisplaySize>,
    resize_pending: bool,
    scale_factor: f64,
}

impl GpuState {
    fn resize_before_draw(&mut self, window: &Window, flux: &mut Flux) {
        if !self.resize_pending {
            return;
        }
        self.resize_pending = false;
        // ScaleFactorChanged can precede the OS's final Resized event. Read
        // the current physical dimensions after both events, before drawing.
        let physical = window.inner_size();
        let next = DisplaySize::from_physical(
            physical.width,
            physical.height,
            self.scale_factor,
            self.device.limits().max_texture_dimension_2d,
        );
        if self.size == next {
            return;
        }
        if let Some(size) = next {
            if self.size.is_none()
                || self.config.width != size.physical_width
                || self.config.height != size.physical_height
            {
                self.config.width = size.physical_width;
                self.config.height = size.physical_height;
                self.window_surface.configure(&self.device, &self.config);
            }
            flux.resize(
                &self.device,
                &self.command_queue,
                size.logical_width,
                size.logical_height,
                size.physical_width,
                size.physical_height,
            );
        }
        self.size = next;
    }
}

struct FluxApp {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    app: Option<App>,
    start: std::time::Instant,
    settings: Arc<Settings>,
    clock_settings: config::ClockSettings,
    palette: Option<RgbaImage>,
    options: Options,
    pointer: Option<winit::dpi::PhysicalPosition<f64>>,
    frames: u64,
    parent: Option<xscreensaver::Parent>,
}

#[derive(Parser, Debug)]
#[command(
    name = "linuxdrift",
    version,
    about = "LinuxDrift — native Linux screensaver"
)]
struct Options {
    /// Open the settings window; save to the user's XDG configuration directory.
    #[arg(long, conflicts_with_all = ["preview", "check_config", "print_config"])]
    config: bool,
    /// Render inside the XScreenSaver window (or the X11 root window).
    #[arg(long)]
    root: bool,
    /// Embed in this X11 window (decimal or hexadecimal).
    #[arg(long)]
    window_id: Option<String>,
    /// Override a saved setting for this invocation: --set lineLength=300
    #[arg(long = "set", value_name = "KEY=JSON")]
    overrides: Vec<String>,
    /// Show the saved clock (override enabled only).
    #[arg(long, conflicts_with = "no_clock")]
    clock: bool,
    /// Hide the clock for this invocation.
    #[arg(long)]
    no_clock: bool,
    /// Apply a performance preset for this invocation (see --list-presets).
    #[arg(long)]
    preset: Option<String>,
    /// List the ten performance presets as JSON.
    #[arg(long)]
    list_presets: bool,
    /// Override the palette for this invocation.
    #[arg(long, value_parser = ["original", "plasma", "poolside", "gumdrop", "silver", "charcoal", "glitter", "andromeda", "verdant", "freedom"])]
    palette: Option<String>,
    /// Run in a resizable window (Escape to close).
    #[arg(long)]
    preview: bool,
    /// Validate the saved settings and palette, without opening a window.
    #[arg(long)]
    check_config: bool,
    /// Print the effective configuration as JSON.
    #[arg(long)]
    print_config: bool,
    /// Override automatic session detection.
    #[arg(long, default_value = "auto", value_parser = ["auto", "x11", "wayland"])]
    backend: String,
    /// Exit after this many seconds (useful for smoke tests).
    #[arg(long, value_parser = clap::value_parser!(u64).range(1..))]
    duration: Option<u64>,
}

fn select_backend(
    requested: &str,
    session: &str,
    wayland: bool,
    x11: bool,
) -> config::Result<&'static str> {
    match requested {
        "x11" => Ok("x11"),
        "wayland" => Ok("wayland"),
        _ if session == "wayland" && wayland => Ok("wayland"),
        _ if session == "x11" && x11 => Ok("x11"),
        _ if wayland => Ok("wayland"),
        _ if x11 => Ok("x11"),
        _ => Err("No graphical session: neither WAYLAND_DISPLAY nor DISPLAY is set".into()),
    }
}

fn main() {
    if let Err(error) = execute() {
        eprintln!("LinuxDrift: {error}");
        std::process::exit(1);
    }
}

fn execute() -> config::Result<()> {
    // XScreenSaver historically uses single-dash long options.
    let arguments = std::env::args_os().map(|arg| match arg.to_str() {
        Some("-root") => "--root".into(),
        Some("-window-id") => "--window-id".into(),
        _ => arg,
    });
    let options = Options::parse_from(arguments);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("error")).init();
    if options.list_presets {
        println!("{}", serde_json::to_string_pretty(&config::presets())?);
        return Ok(());
    }
    let path = config::path()?;
    let mut settings = config::load(&path)?;
    if options.config {
        return config::edit(&path, settings);
    }
    if let Some(id) = &options.preset {
        config::apply_preset(&mut settings, id)?;
    }
    let mut value = serde_json::to_value(&settings)?;
    for item in &options.overrides {
        let (key, input) = item.split_once('=').ok_or("Use --set KEY=JSON")?;
        if value.get(key).is_none() {
            return Err(format!("Unknown setting: {key}").into());
        }
        value[key] = serde_json::from_str(input)?;
    }
    if let Some(name) = &options.palette {
        value["colorMode"] = match name.as_str() {
            "original" => serde_json::json!({"Preset": "Original"}),
            "plasma" => serde_json::json!({"Preset": "Plasma"}),
            "poolside" => serde_json::json!({"Preset": "Poolside"}),
            "andromeda" => serde_json::json!({"ImageFile": "builtin:galaxy"}),
            name => serde_json::json!({"ImageFile": format!("builtin:{name}")}),
        };
    }
    if !options.overrides.is_empty() {
        value["performancePreset"] = "custom".into();
    }
    settings = config::parse(&serde_json::to_vec(&value)?)?;
    if options.clock {
        settings.clock.enabled = true;
    }
    if options.no_clock {
        settings.clock.enabled = false;
    }
    if options.print_config {
        println!("{}", serde_json::to_string_pretty(&settings)?);
        return Ok(());
    }
    let palette = config::palette(&settings)?;
    if options.check_config {
        println!("Valid: {}", path.display());
        return Ok(());
    }
    // The legacy Freedom enum has no native wheel; use its embedded image.
    if settings.color_mode
        == flux::settings::ColorMode::Preset(flux::settings::ColorPreset::Freedom)
    {
        settings.color_mode = flux::settings::ColorMode::ImageFile("builtin:freedom".into());
    }
    let parent_id = options
        .window_id
        .clone()
        .or_else(|| std::env::var("XSCREENSAVER_WINDOW").ok());
    let embedded = options.root || parent_id.is_some();
    if embedded && options.backend == "wayland" {
        return Err("XScreenSaver embedding requires X11; omit --backend wayland".into());
    }
    let parent = if embedded {
        Some(xscreensaver::Parent::open(parent_id.as_deref())?)
    } else {
        None
    };
    let backend = select_backend(
        if embedded { "x11" } else { &options.backend },
        &std::env::var("XDG_SESSION_TYPE").unwrap_or_default(),
        std::env::var_os("WAYLAND_DISPLAY").is_some_and(|s| !s.is_empty()),
        std::env::var_os("DISPLAY").is_some_and(|s| !s.is_empty()),
    )?;
    log::info!("LinuxDrift display backend: {backend}");
    let mut builder = EventLoop::builder();
    match backend {
        "wayland" => {
            winit::platform::wayland::EventLoopBuilderExtWayland::with_wayland(&mut builder);
        }
        _ => {
            winit::platform::x11::EventLoopBuilderExtX11::with_x11(&mut builder);
        }
    }
    let event_loop = builder.build()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    let mut flux_app = FluxApp {
        window: None,
        gpu: None,
        app: None,
        start: std::time::Instant::now(),
        clock_settings: settings.clock,
        settings: Arc::new(settings.flux),
        palette,
        options,
        pointer: None,
        frames: 0,
        parent,
    };
    event_loop.run_app(&mut flux_app)?;
    if flux_app.frames == 0 {
        return Err("No frames were rendered".into());
    }
    log::info!("Rendered {} frames", flux_app.frames);
    Ok(())
}

impl ApplicationHandler for FluxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("LinuxDrift")
            .with_decorations(self.options.preview)
            // A non-resizable Wayland window sends equal min/max size hints.
            // Mutter then drops fullscreen to honour those fixed dimensions.
            // Let the compositor control the saver size; only previews get a
            // preferred initial window size.
            .with_resizable(true)
            .with_fullscreen(if self.options.preview {
                None
            } else {
                Some(winit::window::Fullscreen::Borderless(None))
            });

        let window_attributes = if self.options.preview {
            window_attributes.with_inner_size(winit::dpi::LogicalSize::new(1280, 800))
        } else {
            window_attributes
        };

        let window_attributes = if let Some(parent) = &self.parent {
            use winit::platform::x11::WindowAttributesExtX11;
            window_attributes
                .with_fullscreen(None)
                .with_decorations(false)
                .with_active(false)
                .with_inner_size(winit::dpi::PhysicalSize::new(parent.width, parent.height))
                .with_embed_parent_window(parent.id.into())
        } else {
            window_attributes
        };
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        window.set_cursor_visible(self.options.preview);

        let wgpu_instance = wgpu::Instance::default();
        let window_surface = wgpu_instance.create_surface(window.clone()).unwrap();
        let adapter =
            pollster::block_on(wgpu_instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&window_surface),
                apply_limit_buckets: false,
            }))
            .expect("Failed to find an appropriate adapter");

        let limits = wgpu::Limits::default().using_resolution(adapter.limits());

        let caps = flux::BackendCaps {
            float32_filterable: adapter
                .features()
                .contains(wgpu::Features::FLOAT32_FILTERABLE),
        };
        log::info!("Backend caps: {:?}", caps);

        let mut features = wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
        if caps.float32_filterable {
            features |= wgpu::Features::FLOAT32_FILTERABLE;
        }

        let (device, command_queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: features,
                required_limits: limits,
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
            }))
            .expect("Failed to create device");

        let swapchain_capabilities = window_surface.get_capabilities(&adapter);
        let surface_output = get_preferred_surface_output(&swapchain_capabilities)
            .expect("Surface does not support a renderer-compatible color space");
        let display_hdr_info = window_surface.display_hdr_info(&adapter);
        log::info!(
            "Surface output: format={:?}, color_space={:?}, hdr={}, headroom={:?}",
            surface_output.format,
            surface_output.color_space,
            surface_output.color_space.is_hdr(),
            display_hdr_info.tone_map_headroom(),
        );
        log::debug!(
            "Surface format capabilities: {:?}",
            swapchain_capabilities.format_capabilities
        );

        let physical_size = window.inner_size();
        let scale_factor = window.scale_factor();
        let size = DisplaySize::from_physical(
            physical_size.width,
            physical_size.height,
            scale_factor,
            device.limits().max_texture_dimension_2d,
        );
        let initial_size = size.unwrap_or(DisplaySize::from_logical(1, 1, 1.0, 1).unwrap());
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_output.format,
            width: initial_size.physical_width,
            height: initial_size.physical_height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: swapchain_capabilities.alpha_modes[0],
            view_formats: vec![],
            color_space: surface_output.color_space,
        };

        if size.is_some() {
            window_surface.configure(&device, &config);
        }
        let settings = Arc::clone(&self.settings);
        let mut flux = Flux::new(
            &device,
            &command_queue,
            surface_output.format,
            initial_size.logical_width,
            initial_size.logical_height,
            initial_size.physical_width,
            initial_size.physical_height,
            caps,
            &Arc::clone(&settings),
        )
        .unwrap();

        if let Some(image) = self.palette.take() {
            flux.sample_colors_from_image(&device, &command_queue, &image);
        }
        window.set_visible(true);

        let clock = self.clock_settings.enabled.then(|| {
            clock::ClockOverlay::new(&device, surface_output.format, self.clock_settings.clone())
        });
        self.app = Some(App { flux, clock });

        self.gpu = Some(GpuState {
            device,
            command_queue,
            window_surface,
            config,
            size,
            resize_pending: false,
            scale_factor,
        });

        self.window = Some(window);
        self.start = std::time::Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let (Some(window), Some(gpu), Some(app)) =
            (self.window.as_ref(), self.gpu.as_mut(), self.app.as_mut())
        else {
            return;
        };

        if window_id != window.id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(KeyCode::Escape),
                        state: ElementState::Released,
                        ..
                    },
                ..
            } if self.parent.is_none() => event_loop.exit(),
            WindowEvent::KeyboardInput { .. }
            | WindowEvent::MouseInput { .. }
            | WindowEvent::MouseWheel { .. }
            | WindowEvent::Touch(_)
                if !self.options.preview && self.parent.is_none() =>
            {
                if self.start.elapsed().as_millis() > 700 {
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. }
                if !self.options.preview && self.parent.is_none() =>
            {
                if self.start.elapsed().as_millis() > 700 {
                    if let Some(old) = self.pointer {
                        if (position.x - old.x).abs() + (position.y - old.y).abs() > 4.0 {
                            event_loop.exit();
                        }
                    } else {
                        self.pointer = Some(position);
                    }
                } else {
                    self.pointer = Some(position);
                }
            }
            WindowEvent::Resized(size) => {
                log::info!(
                    "Window configured: {}x{}, fullscreen={}",
                    size.width,
                    size.height,
                    window.fullscreen().is_some()
                );
                gpu.resize_pending = true;
                gpu.scale_factor = window.scale_factor();
                window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                gpu.resize_pending = true;
                gpu.scale_factor = scale_factor;
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                gpu.resize_before_draw(window, &mut app.flux);
                if gpu.size.is_none() {
                    return;
                }
                let frame = match gpu.window_surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(frame)
                    | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        window.request_redraw();
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Outdated => {
                        gpu.window_surface.configure(&gpu.device, &gpu.config);
                        window.request_redraw();
                        return;
                    }
                    wgpu::CurrentSurfaceTexture::Lost => {
                        gpu.window_surface.configure(&gpu.device, &gpu.config);
                        window.request_redraw();
                        return;
                    }
                    status => {
                        log::error!("Surface failure: {status:?}");
                        event_loop.exit();
                        return;
                    }
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let mut encoder =
                    gpu.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("flux:render"),
                        });

                app.flux.animate(
                    &gpu.device,
                    &gpu.command_queue,
                    &mut encoder,
                    &view,
                    None,
                    self.start.elapsed().as_secs_f64() * 1000.0,
                );

                if let Some(clock) = &mut app.clock {
                    clock.draw(
                        &gpu.device,
                        &gpu.command_queue,
                        &mut encoder,
                        &view,
                        gpu.config.width,
                        gpu.config.height,
                        self.start.elapsed(),
                    );
                }
                gpu.command_queue.submit(Some(encoder.finish()));
                window.pre_present_notify();
                gpu.command_queue.present(frame);
                self.frames += 1;
                if self
                    .options
                    .duration
                    .is_some_and(|seconds| self.start.elapsed().as_secs() >= seconds)
                {
                    event_loop.exit();
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(parent) = &mut self.parent {
            match parent.poll() {
                Ok(Some(true)) => {
                    if let Some(window) = &self.window {
                        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(
                            parent.width,
                            parent.height,
                        ));
                    }
                }
                Ok(Some(false)) | Err(_) => {
                    event_loop.exit();
                    return;
                }
                Ok(None) => (),
            }
        }
        if let (Some(window), Some(gpu)) = (self.window.as_ref(), self.gpu.as_ref()) {
            if gpu.size.is_some() || gpu.resize_pending {
                window.request_redraw();
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SurfaceOutput {
    format: wgpu::TextureFormat,
    color_space: wgpu::SurfaceColorSpace,
}

fn get_preferred_surface_output(capabilities: &wgpu::SurfaceCapabilities) -> Option<SurfaceOutput> {
    // Flux's existing output values are sRGB-encoded. Prefer an encoded
    // extended-sRGB surface where available (Metal and some Vulkan drivers),
    // preserving the current appearance and values above SDR white.
    let encoded_hdr = SurfaceOutput {
        format: wgpu::TextureFormat::Rgba16Float,
        color_space: wgpu::SurfaceColorSpace::ExtendedSrgb,
    };
    if supports_surface_output(capabilities, encoded_hdr) {
        return Some(encoded_hdr);
    }

    // Linear scRGB and PQ/HLG need an explicit output conversion pass. Until
    // Flux has one, using those spaces would alter the existing colors.
    // Preserve the existing SDR output convention: non-sRGB formats contain an
    // sRGB-encoded signal, while *Srgb formats perform the encoding on store.
    // Every candidate is paired with an explicitly advertised color space.
    let sdr_formats = [
        wgpu::TextureFormat::Rgb10a2Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ];
    sdr_formats.into_iter().find_map(|format| {
        let output = SurfaceOutput {
            format,
            color_space: wgpu::SurfaceColorSpace::Srgb,
        };
        supports_surface_output(capabilities, output).then_some(output)
    })
}

fn supports_surface_output(
    capabilities: &wgpu::SurfaceCapabilities,
    output: SurfaceOutput,
) -> bool {
    capabilities
        .color_spaces(output.format)
        .contains(output.color_space.to_color_spaces().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_native_session_even_with_xwayland() {
        assert_eq!(
            select_backend("auto", "wayland", true, true).unwrap(),
            "wayland"
        );
        assert_eq!(select_backend("auto", "x11", true, true).unwrap(), "x11");
        assert_eq!(select_backend("auto", "", true, true).unwrap(), "wayland");
        assert_eq!(select_backend("auto", "", false, true).unwrap(), "x11");
        assert_eq!(select_backend("x11", "wayland", true, true).unwrap(), "x11");
        assert!(select_backend("auto", "", false, false).is_err());
    }

    fn capabilities(
        formats: &[(wgpu::TextureFormat, wgpu::SurfaceColorSpaces)],
    ) -> wgpu::SurfaceCapabilities {
        wgpu::SurfaceCapabilities {
            formats: Vec::new(),
            format_capabilities: formats
                .iter()
                .map(|&(format, color_spaces)| wgpu::SurfaceFormatCapabilities {
                    format,
                    color_spaces,
                })
                .collect(),
            present_modes: Vec::new(),
            alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
            usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
        }
    }

    #[test]
    fn falls_back_to_sdr_when_only_linear_hdr_is_advertised() {
        let capabilities = capabilities(&[
            (
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::SurfaceColorSpaces::SRGB,
            ),
            (
                wgpu::TextureFormat::Rgba16Float,
                wgpu::SurfaceColorSpaces::EXTENDED_SRGB_LINEAR,
            ),
        ]);

        assert_eq!(
            get_preferred_surface_output(&capabilities),
            Some(SurfaceOutput {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color_space: wgpu::SurfaceColorSpace::Srgb,
            })
        );
    }

    #[test]
    fn prefers_encoded_hdr_without_changing_existing_colors() {
        let capabilities = capabilities(&[
            (
                wgpu::TextureFormat::Rgba16Float,
                wgpu::SurfaceColorSpaces::EXTENDED_SRGB,
            ),
            (
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::SurfaceColorSpaces::SRGB,
            ),
        ]);

        assert_eq!(
            get_preferred_surface_output(&capabilities),
            Some(SurfaceOutput {
                format: wgpu::TextureFormat::Rgba16Float,
                color_space: wgpu::SurfaceColorSpace::ExtendedSrgb,
            })
        );
    }

    #[test]
    fn falls_back_to_supported_sdr_pair() {
        let capabilities = capabilities(&[
            (
                wgpu::TextureFormat::Rgb10a2Unorm,
                wgpu::SurfaceColorSpaces::BT2100_PQ,
            ),
            (
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::SurfaceColorSpaces::SRGB,
            ),
        ]);

        assert_eq!(
            get_preferred_surface_output(&capabilities),
            Some(SurfaceOutput {
                format: wgpu::TextureFormat::Bgra8Unorm,
                color_space: wgpu::SurfaceColorSpace::Srgb,
            })
        );
    }
}
