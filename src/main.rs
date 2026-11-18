//! Genesis V0.1 mobile viewer. All dynamics live in the 'genesis' library.
use genesis::{Pyramid, Sample, Universe};
use macroquad::prelude::*;

const WIDTH: usize = 256;
const HEIGHT: usize = 256;
const DEPTH: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode { Matter, Energy, Contacts }
impl Mode {
    fn next(self) -> Self {
        match self { Self::Matter => Self::Energy, Self::Energy => Self::Contacts, Self::Contacts => Self::Matter }
    }
    fn label(self) -> &'static str {
        match self { Self::Matter => "MATTER", Self::Energy => "ENERGY", Self::Contacts => "CONTACTS" }
    }
}

fn config() -> Conf {
    Conf {
        window_title: "Genesis - An Artificial Universe".into(),
        window_width: 450,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

fn lod_for(zoom: f32) -> usize {
    if zoom < 1.4 { 4 }
    else if zoom < 2.5 { 3 }
    else if zoom < 5.0 { 2 }
    else if zoom < 9.0 { 1 }
    else { 0 }
}

fn color_for(s: &Sample, mode: Mode) -> Color {
    let occupied = s.occupied() as f32;
    let total = s.cells().max(1) as f32;
    match mode {
        Mode::Matter => {
            if occupied == 0.0 { return Color::from_rgba(14, 25, 38, 255); }
            let mut dominant = 1;
            for k in 2..4 {
                if s.counts[k] > s.counts[dominant] { dominant = k; }
            }
            let fill = occupied / total;
            let base: [u8; 3] = match dominant {
                1 => [61, 166, 218], 2 => [243, 169, 88], _ => [96, 213, 162],
            };
            let mix = (0.26 + fill * 0.74).min(1.0);
            Color::from_rgba(
                (base[0] as f32 * mix) as u8,
                (base[1] as f32 * mix) as u8,
                (base[2] as f32 * mix) as u8,
                255,
            )
        }
        Mode::Energy => {
            let avg = (s.heat as f32 / total / 95.0).clamp(0.0, 1.0);
            Color::new(0.10 + avg * 0.86, 0.15 + avg * 0.48, 0.37 - avg * 0.23, 1.0)
        }
        Mode::Contacts => {
            let fraction = (s.contacts as f32 / (total * 6.0)).clamp(0.0, 1.0);
            Color::new(0.07 + fraction * 0.53, 0.18 + fraction * 0.76, 0.22 + fraction * 0.37, 1.0)
        }
    }
}

/// Paint the *same* layer using a selected observational scale.
/// Every displayed pixel comes from an exact summary of elementary cells.
fn paint(image: &mut Image, world: &Universe, z: usize, mode: Mode, lod: usize) {
    let hierarchy = Pyramid::from_layer(world, z);
    let level = hierarchy.at(lod);
    let block = 1_usize << lod.min(hierarchy.levels.len() - 1);
    for py in 0..world.height {
        for px in 0..world.width {
            let sx = (px / block).min(level.width - 1);
            let sy = (py / block).min(level.height - 1);
            let s = &level.samples[sy * level.width + sx];
            image.set_pixel(px as u32, py as u32, color_for(s, mode));
        }
    }
}

fn control(rect: Rect, title: &str, active: bool) {
    let bg = if active { Color::from_rgba(48, 119, 142, 255) }
        else { Color::from_rgba(34, 52, 71, 255) };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    let size = 15.0_f32.min(rect.w / (title.len() as f32 * 0.49 + 1.0));
    let dims = measure_text(title, None, size as u16, 1.0);
    draw_text(title,
        rect.x + (rect.w - dims.width) * 0.5,
        rect.y + (rect.h + dims.height) * 0.5 - 2.0,
        size, WHITE);
}

fn tap_point() -> Option<Vec2> {
    // Explicit touch support; browser mouse events work as a fallback.
    for t in touches() {
        if t.phase == TouchPhase::Started {
            return Some(t.position);
        }
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        Some(vec2(x, y))
    } else { None }
}

#[macroquad::main(config)]
async fn main() {
    let mut seed = 1718_u64;
    let mut universe = Universe::seeded(WIDTH, HEIGHT, DEPTH, seed);
    let mut baseline = universe.audit();
    let mut audit = baseline;
    let mut last_audited_tick = 0_u64;
    let mut running = true;
    let mut speed = 4_usize;
    let mut layer = DEPTH / 2;
    let mut mode = Mode::Matter;
    let mut stack_view = false;
    let mut zoom = 1.0_f32;
    let mut pan = Vec2::ZERO;
    let mut accumulator = 0.0_f32;
    let mut dirty = true;
    let mut prev_lod = lod_for(zoom);
    let mut inspected: Option<(usize, usize, usize)> = None;
    let mut previous_pointer: Option<Vec2> = None;
    let mut previous_pinch: Option<f32> = None;

    // A maximum of three reusable GPU textures: current layer and two lower layers.
    let mut images: Vec<Image> = (0..3)
        .map(|_| Image::gen_image_color(WIDTH as u16, HEIGHT as u16, BLACK)).collect();
    let textures: Vec<Texture2D> = images.iter().map(Texture2D::from_image).collect();

    loop {
        let sw = screen_width();
        let sh = screen_height();
        let map_box = (sw - 20.0).min((sh - 275.0).max(140.0));
        let top = 99.0;
        let controls_top = sh - 150.0;
        let zoom_pixel_size = map_box * zoom;
        let origin = vec2((sw - zoom_pixel_size) * 0.5 + pan.x,
                          top + (controls_top - top - zoom_pixel_size).max(0.0) * 0.5 + pan.y);

        // Fixed tick rate, independent of drawing FPS. Bound the catch-up work on phones.
        if running {
            accumulator += get_frame_time().min(0.15);
            let interval = 1.0 / speed as f32;
            let mut ticks_this_frame = 0;
            while accumulator >= interval && ticks_this_frame < 3 {
                universe.step();
                accumulator -= interval;
                ticks_this_frame += 1;
                dirty = true;
            }
            if ticks_this_frame == 3 { accumulator = accumulator.min(interval); }
        } else {
            accumulator = 0.0;
        }

        let touch_list = touches();
        if touch_list.len() >= 2 {
            let d = touch_list[0].position.distance(touch_list[1].position);
            if let Some(last) = previous_pinch {
                if last > 5.0 {
                    zoom = (zoom * d / last).clamp(0.65, 18.0);
                }
            }
            previous_pinch = Some(d);
            previous_pointer = None;
        } else {
            previous_pinch = None;
            let pointer = if let Some(t) = touch_list.first() {
                if t.phase == TouchPhase::Ended || t.phase == TouchPhase::Cancelled {
                    None
                } else { Some(t.position) }
            } else if is_mouse_button_down(MouseButton::Left) {
                let (x, y) = mouse_position();
                Some(vec2(x, y))
            } else { None };

            if let (Some(now), Some(last)) = (pointer, previous_pointer) {
                if now.y < controls_top && last.y < controls_top {
                    pan += now - last;
                }
            }
            previous_pointer = pointer;
        }
        let wheel = mouse_wheel().1;
        if wheel.abs() > 0.01 {
            zoom = (zoom * (1.0 + wheel * 0.12).max(0.4)).clamp(0.65, 18.0);
        }
        let lod = lod_for(zoom);
        if lod != prev_lod { dirty = true; prev_lod = lod; }

        // Controls sit below the simulation canvas for comfortable thumb use.
        let side = 10.0;
        let gap = 5.0;
        let button_w = ((sw - side * 2.0) - gap * 3.0) / 4.0;
        let bh = 42.0;
        let mut buttons = Vec::with_capacity(12);
        for row in 0..3 {
            for col in 0..4 {
                buttons.push(Rect::new(side + col as f32 * (button_w + gap),
                    controls_top + row as f32 * (bh + 5.0), button_w, bh));
            }
        }
        let labels = [
            if running { "PAUSE" } else { "RUN" }, "STEP", "Z -", "Z +",
            "ZOOM -", "ZOOM +", mode.label(), if stack_view { "2.5D" } else { "2D" },
            "RESET", "SPEED", "CENTER", "AUDIT",
        ];

        let tapped = tap_point();
        if let Some(point) = tapped {
            let pressed = buttons.iter().position(|r| r.contains(point));
            match pressed {
                Some(0) => running = !running,
                Some(1) => { universe.step(); dirty = true; },
                Some(2) => { layer = layer.saturating_sub(1); dirty = true; },
                Some(3) => { layer = (layer + 1).min(DEPTH - 1); dirty = true; },
                Some(4) => zoom = (zoom / 1.7).max(0.65),
                Some(5) => zoom = (zoom * 1.7).min(18.0),
                Some(6) => { mode = mode.next(); dirty = true; },
                Some(7) => { stack_view = !stack_view; dirty = true; },
                Some(8) => {
                    seed = seed.wrapping_add(1);
                    universe = Universe::seeded(WIDTH, HEIGHT, DEPTH, seed);
                    baseline = universe.audit();
                    audit = baseline;
                    last_audited_tick = 0;
                    running = false;
                    inspected = None;
                    dirty = true;
                }
                Some(9) => speed = match speed { 1 => 4, 4 => 12, _ => 1 },
                Some(10) => { pan = Vec2::ZERO; zoom = 1.0; },
                Some(11) => audit = universe.audit(),
                _ => {
                    let tx = (point.x - origin.x) / zoom_pixel_size;
                    let ty = (point.y - origin.y) / zoom_pixel_size;
                    if (0.0..1.0).contains(&tx) && (0.0..1.0).contains(&ty) {
                        inspected = Some(((tx * WIDTH as f32) as usize,
                            (ty * HEIGHT as f32) as usize, layer));
                    }
                }
            }
        }
        let actual_lod = lod_for(zoom);
        if actual_lod != prev_lod { dirty = true; prev_lod = actual_lod; }

        if dirty {
            for slot in 0..3 {
                let distance = 2 - slot;
                if (stack_view || slot == 2) && layer >= distance {
                    let z = layer - distance;
                    paint(&mut images[slot], &universe, z, mode, actual_lod);
                    textures[slot].update(&images[slot]);
                }
            }
            dirty = false;
        }

        if universe.tick.saturating_sub(last_audited_tick) >= 16 {
            // Audits are deliberately periodic, not every rendered frame.
            audit = universe.audit();
            last_audited_tick = universe.tick;
        }
        clear_background(Color::from_rgba(12, 20, 32, 255));
        draw_text("GENESIS", 15.0, 37.0, 34.0, WHITE);
        draw_text(&format!("Tick {}   |   Z {}/{}   |   {} TPS",
            universe.tick, layer + 1, DEPTH, speed),
            16.0, 61.0, 18.0, LIGHTGRAY);
        let drift = audit.total_energy - baseline.total_energy;
        draw_text(&format!("Conserved energy: {}   Drift: {}",
            audit.total_energy, drift),
            16.0, 82.0, 15.0,
            if drift == 0 { GREEN } else { RED });

        // 2.5D is a camera: offset semi-transparent slices, never new physics.
        if stack_view {
            for slot in 0..3 {
                let distance = 2 - slot;
                if layer < distance { continue; }
                let offset = distance as f32;
                let alpha = if distance == 0 { 1.0 } else if distance == 1 { 0.47 } else { 0.23 };
                draw_texture_ex(&textures[slot], origin.x + offset * 14.0,
                    origin.y - offset * 12.0,
                    Color::new(1.0, 1.0, 1.0, alpha),
                    DrawTextureParams { dest_size: Some(vec2(zoom_pixel_size, zoom_pixel_size)),
                        ..Default::default() });
            }
        } else {
            draw_texture_ex(&textures[2], origin.x, origin.y, WHITE,
                DrawTextureParams { dest_size: Some(vec2(zoom_pixel_size, zoom_pixel_size)),
                    ..Default::default() });
        }

        if let Some((x, y, z)) = inspected {
            let cell = universe.cells[universe.index(x, y, z)];
            let info = format!("Cell ({},{},{})  Matter={}  Heat={}", x, y, z, cell.matter, cell.heat);
            draw_rectangle(8.0, controls_top - 25.0, (sw - 16.0).max(1.0), 23.0, Color::from_rgba(10, 25, 40, 235));
            draw_text(&info, 12.0, controls_top - 8.0, 14.0, WHITE);
        }
        draw_text(&format!("Scale: 1 cell per {}x{} summary | {}",
            1usize << actual_lod, 1usize << actual_lod, if running { "RUNNING" } else { "PAUSED" }),
            14.0, controls_top - 31.0, 14.0, LIGHTGRAY);
        for i in 0..buttons.len() {
            control(buttons[i], labels[i], i == 0 && running || i == 7 && stack_view);
        }
        next_frame().await;
    }
}
