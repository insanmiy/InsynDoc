#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod document;
mod editor;
mod icons;
mod page_layout;
mod prefs;
mod state;
mod theme;
mod ui;
#[cfg(test)]
mod visual_tests;
mod workspace;

use std::sync::Arc;

use egui::Context;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowId};

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<Arc<wgpu::Device>>,
    queue: Option<Arc<wgpu::Queue>>,
    config: Option<wgpu::SurfaceConfiguration>,
    egui_ctx: Context,
    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    app_state: state::AppState,
    logo: Option<egui::TextureHandle>,
    logo_bright: Option<egui::TextureHandle>,
    workspace: workspace::Workspace,
    resolved_theme: theme::Theme,
    next_repaint: Option<std::time::Instant>,
}

const INTER_TTF: &[u8] = include_bytes!("../fonts/Inter-Regular.ttf");
const INTER_MEDIUM_TTF: &[u8] = include_bytes!("../fonts/Inter-Medium.ttf");
const INTER_SEMIBOLD_TTF: &[u8] = include_bytes!("../fonts/Inter-SemiBold.ttf");
const LOGO_PNG: &[u8] = include_bytes!("../../public/branding/insyndoc-logo.png");

fn load_logo(ctx: &Context, brighten: bool) -> Option<egui::TextureHandle> {
    let image = image::load_from_memory_with_format(LOGO_PNG, image::ImageFormat::Png).ok()?;
    let mut rgba = image.to_rgba8();
    if brighten {
        // Approximate the CSS dark-theme filter `brightness(2) saturate(0.8)`:
        // push channels toward white, then pull saturation down ~20%.
        for px in rgba.pixels_mut() {
            let r = f32::from(px[0]) * 2.0;
            let g = f32::from(px[1]) * 2.0;
            let b = f32::from(px[2]) * 2.0;
            let gray = (r + g + b) / 3.0;
            px[0] = (gray + (r - gray) * 0.8).min(255.0) as u8;
            px[1] = (gray + (g - gray) * 0.8).min(255.0) as u8;
            px[2] = (gray + (b - gray) * 0.8).min(255.0) as u8;
        }
    }
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], rgba.as_raw());
    Some(ctx.load_texture(
        if brighten { "logo_bright" } else { "logo" },
        color_image,
        egui::TextureOptions::LINEAR,
    ))
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            egui_ctx: Context::default(),
            egui_state: None,
            egui_renderer: None,
            app_state: state::AppState::default(),
            logo: None,
            logo_bright: None,
            workspace: workspace::Workspace::default(),
            resolved_theme: theme::Theme::Light,
            next_repaint: None,
        }
    }

    fn init(&mut self, event_loop: &ActiveEventLoop) {
        theme::install_fonts(
            &self.egui_ctx,
            INTER_TTF,
            INTER_MEDIUM_TTF,
            INTER_SEMIBOLD_TTF,
        );
        theme::apply_theme(&self.egui_ctx, theme::Theme::Light);
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("InsynDoc")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
                )
                .expect("failed to create window"),
        );

        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });
        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("failed to create surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no suitable GPU adapter");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
                .expect("failed to create device");
        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        let native_ppp = window.scale_factor() as f32;
        let max_texture_side = device.limits().max_texture_dimension_2d;
        let egui_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(native_ppp),
            None,
            Some(max_texture_side as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
        self.logo = load_logo(&self.egui_ctx, false);
        self.logo_bright = load_logo(&self.egui_ctx, true);

        self.app_state.preferences = prefs::load(&prefs::app_data_dir());
        self.workspace.startup(&mut self.app_state);

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);
        self.apply_preferences();
        self.egui_ctx.request_repaint();
        if let Some(window) = self.window.as_ref() { window.request_redraw(); }
    }

    fn apply_preferences(&mut self) {
        let theme = match self.app_state.preferences.theme {
            state::ThemePref::Light => theme::Theme::Light,
            state::ThemePref::Dark => theme::Theme::Dark,
            state::ThemePref::System => {
                match self.window.as_ref().and_then(|window| window.theme()) {
                    Some(winit::window::Theme::Dark) => theme::Theme::Dark,
                    _ => theme::Theme::Light,
                }
            }
        };
        self.resolved_theme = theme;
        theme::apply_theme(&self.egui_ctx, theme);
        if let Some(window) = self.window.as_ref() {
            window.set_theme(match self.app_state.preferences.theme {
                state::ThemePref::System => None,
                state::ThemePref::Dark => Some(winit::window::Theme::Dark),
                state::ThemePref::Light => Some(winit::window::Theme::Light),
            });
        }
        prefs::store(&prefs::app_data_dir(), &self.app_state.preferences);
    }

    fn request_close(&mut self) {
        self.workspace.request_close(&mut self.app_state);
        self.egui_ctx.request_repaint();
        if let Some(window) = self.window.as_ref() { window.request_redraw(); }
    }

    /// Execute commands produced by the chrome UI.
    fn execute_ui_commands(&mut self, commands: Vec<ui::UiCommand>) {
        let changed = !commands.is_empty();
        for command in commands {
            match command {
                ui::UiCommand::Command { name, args } | ui::UiCommand::Exec { name, args } => {
                    self.workspace
                        .execute(&self.egui_ctx, &name, &args, &mut self.app_state);
                }
                ui::UiCommand::Quit => self.request_close(),
                ui::UiCommand::SetTitle { name, dirty } => {
                    if let Some(window) = self.window.as_ref() {
                        let prefix = if dirty { "* " } else { "" };
                        let _ = window.set_title(&format!("{}{} | InsynDoc", prefix, name));
                    }
                }
                ui::UiCommand::CheckUpdates => self.check_updates(),
                ui::UiCommand::InstallUpdate => self.install_update(),
                ui::UiCommand::DismissUpdate => {
                    self.app_state.update = None;
                    self.app_state.update_error = None;
                    self.app_state.preferences.update_notifications = false;
                    self.apply_preferences();
                }
                ui::UiCommand::FetchRecent => {
                    self.workspace.recents(&self.egui_ctx, &mut self.app_state)
                }
                ui::UiCommand::ChooseBrowser => self.choose_browser(),
                ui::UiCommand::SavePreferences => self.apply_preferences(),
                ui::UiCommand::ConfirmChoice { choice } => {
                    self.workspace.confirm(choice, &mut self.app_state)
                }
                ui::UiCommand::ClearNotice => self.app_state.surface.notice = None,
            }
        }
        if changed { self.egui_ctx.request_repaint(); }
    }

    fn choose_browser(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Browser", &["exe"])
            .pick_file()
        {
            if let Err(error) = backend::links::validate_browser(&path) {
                self.app_state.link_error = Some(error);
                return;
            }
            self.app_state.preferences.browser_path = Some(path.to_string_lossy().into_owned());
            self.apply_preferences();
        }
    }

    fn check_updates(&mut self) {
        ui::modals::set_update_status(&self.egui_ctx, "unsupported");
        self.app_state.update_error = None;
    }

    fn install_update(&mut self) {
        self.app_state.update_error =
            Some("Update Installation Is Not Yet Available For The Rust App.".into());
    }

    fn keyboard_commands(ctx: &Context, state: &mut state::AppState, out: &mut ui::UiOutput) {
        if state.modal != state::Modal::None || state.renaming {
            return;
        }
        use egui::{Key, Modifiers};
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::F)) {
            out.command_args(
                "focusMode",
                serde_json::json!({"enabled":!state.surface.focus_mode}),
            );
        }
        let shortcuts = [
            (Modifiers::CTRL, Key::N, "newDocument"),
            (Modifiers::CTRL, Key::O, "open"),
            (Modifiers::CTRL | Modifiers::SHIFT, Key::S, "saveAs"),
            (Modifiers::CTRL, Key::S, "save"),
            (Modifiers::CTRL | Modifiers::SHIFT, Key::I, "findReplace"),
            (Modifiers::CTRL, Key::F, "find"),
        ];
        for (modifiers, key, command) in shortcuts {
            if ctx.input_mut(|input| input.consume_key(modifiers, key)) {
                out.command(command);
            }
        }
        for (key, modal) in [
            (Key::G, state::Modal::GoToPage),
            (Key::Comma, state::Modal::Settings),
            (Key::Slash, state::Modal::Shortcuts),
        ] {
            if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL, key)) {
                state.modal = modal;
            }
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL | Modifiers::ALT, Key::C)) {
            out.exec("editorExec", serde_json::json!({"type":"copyFormatting"}));
        }
        if ctx.input_mut(|input| input.consume_key(Modifiers::CTRL | Modifiers::ALT, Key::X)) {
            out.command("checkSpelling");
        }
    }

    fn paint(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };

        // Phase 1: run the egui UI and collect chrome commands.
        let mut ui_out = ui::UiOutput::default();
        let full_output = {
            let Some(state) = self.egui_state.as_mut() else {
                return;
            };
            let raw_input = state.take_egui_input(&window);
            let egui_ctx = self.egui_ctx.clone();
            let app_state = &mut self.app_state;
            let resolved_theme = self.resolved_theme;
            let workspace = &mut self.workspace;
            let logo = match resolved_theme {
                theme::Theme::Dark => self.logo_bright.as_ref().or(self.logo.as_ref()),
                theme::Theme::Light => self.logo.as_ref().or(self.logo_bright.as_ref()),
            }
            .map(|handle| handle.id());
            let mut out = egui_ctx.run(raw_input, |ctx| {
                let assets = ui::ChromeAssets {
                    theme: resolved_theme,
                    logo,
                };
                Self::keyboard_commands(ctx, app_state, &mut ui_out);
                ui::draw(ctx, app_state, &assets, &mut ui_out);
                workspace.editor.show(ctx, app_state, resolved_theme);
                ui_out
                    .commands
                    .append(&mut workspace.editor.queued_commands);
            });
            let platform_output = std::mem::take(&mut out.platform_output);
            state.handle_platform_output(&window, platform_output);
            out
        };
        let has_commands = !ui_out.commands.is_empty();
        let delay = full_output.viewport_output.get(&egui::ViewportId::ROOT).map(|viewport| viewport.repaint_delay).unwrap_or(std::time::Duration::MAX);
        self.next_repaint = if has_commands { Some(std::time::Instant::now()) } else { std::time::Instant::now().checked_add(delay) };
        self.execute_ui_commands(std::mem::take(&mut ui_out.commands));

        // Phase 2: tessellate and render.
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        let Some(device) = self.device.clone() else {
            return;
        };
        let Some(queue) = self.queue.clone() else {
            return;
        };
        let Some(config) = self.config.as_ref() else {
            return;
        };
        let Some(renderer) = self.egui_renderer.as_mut() else {
            return;
        };
        let egui_ctx = self.egui_ctx.clone();

        let pixels_per_point = egui_ctx.pixels_per_point();
        let clipped_primitives = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        for (id, delta) in &full_output.textures_delta.set {
            renderer.update_texture(&device, &queue, *id, delta);
        }
        for id in &full_output.textures_delta.free {
            renderer.free_texture(id);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point,
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("egui encoder"),
        });
        let pre_commands = renderer.update_buffers(
            &device,
            &queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        let frame = match surface.get_current_texture() {
            Ok(frame) => frame,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(&device, config);
                return;
            }
            Err(wgpu::SurfaceError::OutOfMemory) => {
                self.app_state.quitting = true;
                return;
            }
            Err(_) => return,
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut render_pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(
                                theme::UiColors::for_theme(self.resolved_theme).app_bg.r(),
                            ) / 255.0,
                            g: f64::from(
                                theme::UiColors::for_theme(self.resolved_theme).app_bg.g(),
                            ) / 255.0,
                            b: f64::from(
                                theme::UiColors::for_theme(self.resolved_theme).app_bg.b(),
                            ) / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            })
            .forget_lifetime();
        renderer.render(&mut render_pass, &clipped_primitives, &screen_descriptor);
        drop(render_pass);
        queue.submit(
            pre_commands
                .into_iter()
                .chain(std::iter::once(encoder.finish())),
        );
        frame.present();

    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        eprintln!("[native] resumed");
        if self.window.is_none() {
            self.init(event_loop);
            eprintln!("[native] init done");
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if self.window.as_ref().is_none_or(|w| w.id() != window_id) {
            return;
        }
        let Some(window) = self.window.clone() else {
            return;
        };
        let Some(state) = self.egui_state.as_mut() else {
            return;
        };
        let response = state.on_window_event(&window, &event);
        if response.repaint && !matches!(event, WindowEvent::RedrawRequested) { window.request_redraw(); }

        match event {
            WindowEvent::CloseRequested => {
                self.request_close();
                return;
            }
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(device), Some(config)) = (
                    self.surface.as_ref(),
                    self.device.clone(),
                    self.config.as_mut(),
                ) {
                    if size.width > 0 && size.height > 0 {
                        surface.configure(
                            &device,
                            &wgpu::SurfaceConfiguration {
                                width: size.width,
                                height: size.height,
                                ..config.clone()
                            },
                        );
                        *config = wgpu::SurfaceConfiguration {
                            width: size.width,
                            height: size.height,
                            ..config.clone()
                        };
                    }
                }
                self.egui_ctx.request_repaint();
            }
            WindowEvent::ThemeChanged(_) => self.apply_preferences(),
            WindowEvent::RedrawRequested => {
                self.paint();
                if self.app_state.quitting {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(deadline) = self.next_repaint {
            if deadline <= std::time::Instant::now() {
                self.next_repaint = None;
                if let Some(window) = self.window.as_ref() { window.request_redraw(); }
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
            }
        } else {
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }
}

fn main() {
    eprintln!("[native] starting");
    let event_loop = EventLoop::new().expect("failed to create event loop");
    eprintln!("[native] event loop created");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::new();
    let result = event_loop.run_app(&mut app);
    eprintln!("[native] event loop exited: {result:?}");
}
