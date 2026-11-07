use crate::{editor, state, theme, ui};

// Render The Same Native Widgets And GPU Pipeline Used By The Desktop Window.
#[test]
#[ignore = "Writes Native UI Comparison Images Using A GPU Adapter"]
fn render_comparison_images() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .unwrap();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None))
            .unwrap();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);
    let width = 1280;
    let height = 900;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native UI Comparison"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let bytes_per_row = width * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Native UI Pixels"),
        size: (bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let output_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../artifacts/ui");
    std::fs::create_dir_all(&output_dir).unwrap();
    let suffix = std::env::var("INSYNDOC_VISUAL_SUFFIX").unwrap_or_else(|_| "current".into());
    for (name, resolved_theme, modal) in [
        ("dark-editor", theme::Theme::Dark, state::Modal::None),
        ("dark-settings", theme::Theme::Dark, state::Modal::Settings),
        ("light-editor", theme::Theme::Light, state::Modal::None),
        (
            "dark-shortcuts",
            theme::Theme::Dark,
            state::Modal::Shortcuts,
        ),
        ("dark-menu-search", theme::Theme::Dark, state::Modal::None),
        ("dark-find", theme::Theme::Dark, state::Modal::None),
        (
            "dark-settings-actions",
            theme::Theme::Dark,
            state::Modal::Settings,
        ),
    ] {
        let context = egui::Context::default();
        context.style_mut(|style| style.animation_time = 0.0);
        theme::install_fonts(
            &context,
            crate::INTER_TTF,
            crate::INTER_MEDIUM_TTF,
            crate::INTER_SEMIBOLD_TTF,
        );
        theme::apply_theme(&context, resolved_theme);
        let mut state = state::AppState::default();
        state.modal = modal;
        state.preferences.theme = if resolved_theme == theme::Theme::Dark {
            state::ThemePref::Dark
        } else {
            state::ThemePref::Light
        };
        if name == "dark-settings" {
            state.preferences.ruler_unit = state::RulerUnit::Cm;
            state.preferences.word_goal = 10;
        }
        let mut editor = editor::Editor::default();
        if name == "dark-menu-search" {
            editor.execute("searchMenus", &serde_json::Value::Null, &mut state);
        }
        if name == "dark-find" {
            editor
                .document
                .replace_plain_text("Find This Text\nFind More Text");
            editor.execute(
                "findReplace",
                &serde_json::json!({"query":"Find"}),
                &mut state,
            );
            state.surface.dirty = true;
        }
        if name == "dark-settings-actions" {
            ui::modals::set_update_status(&context, "unsupported");
        }
        // Seed one undo entry so the toolbar Undo button renders enabled like
        // the reference captures; the alignment is already left so the
        // document itself is unchanged.
        editor.execute(
            "editorExec",
            &serde_json::json!({"type":"setAlignment","align":"left"}),
            &mut state,
        );
        state.surface.dirty = name == "dark-find";
        state.surface.message = String::new();
        editor.sync_state(&mut state);
        let logo = crate::load_logo(&context, resolved_theme == theme::Theme::Dark);
        for frame_index in 0..20 {
            let output = context.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(width as f32, height as f32),
                    )),
                    time: Some(frame_index as f64 / 60.0),
                    events: if name == "dark-settings-actions" && frame_index == 2 {
                        let pos = egui::pos2(639.0, 192.0);
                        vec![
                            egui::Event::PointerMoved(pos),
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: true,
                                modifiers: egui::Modifiers::NONE,
                            },
                            egui::Event::PointerButton {
                                pos,
                                button: egui::PointerButton::Primary,
                                pressed: false,
                                modifiers: egui::Modifiers::NONE,
                            },
                        ]
                    } else {
                        Vec::new()
                    },
                    ..Default::default()
                },
                |context| {
                    let assets = ui::ChromeAssets {
                        theme: resolved_theme,
                        logo: logo.as_ref().map(|logo| logo.id()),
                    };
                    let mut commands = ui::UiOutput::default();
                    ui::draw(context, &mut state, &assets, &mut commands);
                    editor.show(context, &mut state, resolved_theme);
                },
            );
            for (id, delta) in &output.textures_delta.set {
                renderer.update_texture(&device, &queue, *id, delta);
            }
            let shapes = context.tessellate(output.shapes, output.pixels_per_point);
            let screen = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [width, height],
                pixels_per_point: output.pixels_per_point,
            };
            let mut encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            let commands = renderer.update_buffers(&device, &queue, &mut encoder, &shapes, &screen);
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Native UI Render"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            renderer.render(&mut pass, &shapes, &screen);
            drop(pass);
            encoder.copy_texture_to_buffer(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(bytes_per_row),
                        rows_per_image: Some(height),
                    },
                },
                wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
            );
            queue.submit(
                commands
                    .into_iter()
                    .chain(std::iter::once(encoder.finish())),
            );
            for id in &output.textures_delta.free {
                renderer.free_texture(id);
            }
        }
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                sender.send(result).unwrap()
            });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().unwrap();
        let pixels = buffer.slice(..).get_mapped_range();
        let path = output_dir.join(format!("native-{name}-{suffix}.png"));
        image::save_buffer(&path, &pixels, width, height, image::ColorType::Rgba8).unwrap();
        drop(pixels);
        buffer.unmap();
        println!("Rendered {}", path.display());
    }
}
