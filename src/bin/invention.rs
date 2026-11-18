//! Touch-friendly programmable matter laboratory.
//! Visuals and blueprints operate on cell configuration; only invention::World
//! computes the evolution rules.
use genesis::invention::{Blueprint, State, World, HEIGHT, STAMP_SIZE, WIDTH};
use macroquad::prelude::*;
use quad_storage::STORAGE;

const SLOTS: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Tool {
    Wire, Head, Tail, Erase, Pan, Copy, Stamp,
}
impl Tool {
    fn label(self) -> &'static str {
        match self {
            Tool::Wire => "WIRE", Tool::Head => "PULSE", Tool::Tail => "TAIL",
            Tool::Erase => "ERASE", Tool::Pan => "PAN",
            Tool::Copy => "COPY AREA", Tool::Stamp => "STAMP",
        }
    }
    fn state(self) -> Option<State> {
        match self {
            Tool::Wire => Some(State::Wire), Tool::Head => Some(State::Head),
            Tool::Tail => Some(State::Tail), Tool::Erase => Some(State::Empty),
            _ => None,
        }
    }
}
fn config() -> Conf {
    Conf {
        window_title: "Genesis Invention Lab".into(),
        window_width: 440,
        window_height: 820,
        high_dpi: true,
        ..Default::default()
    }
}
fn state_color(v: State) -> Color {
    match v {
        State::Empty => Color::from_rgba(13, 22, 35, 255),
        State::Wire => Color::from_rgba(218, 153, 73, 255),
        State::Head => Color::from_rgba(72, 202, 252, 255),
        State::Tail => Color::from_rgba(222, 91, 120, 255),
    }
}
fn refresh_texture(image: &mut Image, texture: &Texture2D, world: &World) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            image.set_pixel(x as u32, y as u32, state_color(world.get(x, y)));
        }
    }
    texture.update(image);
}

fn save_blueprint(slot: usize, blueprint: &Blueprint) {
    let key = format!("genesis_invention_blueprint_v1_{}", slot);
    if let Ok(mut store) = STORAGE.lock() {
        store.set(&key, &blueprint.encode());
    }
}
fn load_blueprint(slot: usize) -> Option<Blueprint> {
    let key = format!("genesis_invention_blueprint_v1_{}", slot);
    STORAGE.lock().ok()?.get(&key).and_then(|s| Blueprint::decode(&s))
}
fn button(rect: Rect, label: &str, active: bool) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h,
        if active { Color::from_rgba(38, 114, 125, 255) }
        else { Color::from_rgba(34, 51, 70, 255) });
    let size = (rect.w / (label.len() as f32 * 0.59 + 1.0)).clamp(11.0, 16.0);
    let measure = measure_text(label, None, size as u16, 1.0);
    draw_text(label, rect.x + (rect.w - measure.width) * 0.5,
        rect.y + (rect.h + measure.height) * 0.5 - 2.0, size, WHITE);
}
fn valid_tap() -> Option<Vec2> {
    if let Some(touch) = touches().into_iter().find(|t| t.phase == TouchPhase::Started) {
        return Some(touch.position);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        return Some(vec2(x, y));
    }
    None
}
fn pointer_down() -> Option<Vec2> {
    let list = touches();
    if let Some(t) = list.first() {
        if t.phase != TouchPhase::Ended && t.phase != TouchPhase::Cancelled {
            return Some(t.position);
        }
    }
    if is_mouse_button_down(MouseButton::Left) {
        let (x, y) = mouse_position();
        return Some(vec2(x, y));
    }
    None
}
/// The displayed crop always represents real cells in the authoritative world.
fn view_range(center: Vec2, zoom: f32, viewport: Rect) -> Rect {
    let mut span_w = (46.0 / zoom).clamp(5.0, WIDTH as f32);
    let mut span_h = span_w * viewport.h / viewport.w;
    if span_h > HEIGHT as f32 {
        span_h = HEIGHT as f32;
        span_w = span_h * viewport.w / viewport.h;
    }
    let left = (center.x - span_w / 2.0).clamp(0.0, WIDTH as f32 - span_w);
    let top = (center.y - span_h / 2.0).clamp(0.0, HEIGHT as f32 - span_h);
    Rect::new(left, top, span_w, span_h)
}
fn map_cell(point: Vec2, viewport: Rect, source: Rect) -> (usize, usize) {
    let x = source.x + ((point.x - viewport.x) / viewport.w) * source.w;
    let y = source.y + ((point.y - viewport.y) / viewport.h) * source.h;
    ((x.floor() as usize).min(WIDTH - 1), (y.floor() as usize).min(HEIGHT - 1))
}

#[macroquad::main(config)]
async fn main() {
    let mut world = World::standard();
    world.preset(0); // a *configuration* that demonstrates local signal transmission
    let mut image = Image::gen_image_color(WIDTH as u16, HEIGHT as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    let mut dirty = true;
    let mut playing = true;
    let mut speed = 8_usize;
    let mut accumulator = 0.0_f32;
    let mut tool = Tool::Pan;
    let mut zoom = 1.0_f32;
    let mut center = vec2(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);
    let mut last_pointer: Option<Vec2> = None;
    let mut previous_pinch: Option<f32> = None;
    let mut blueprints: Vec<Option<Blueprint>> = (0..SLOTS).map(load_blueprint).collect();
    let mut slot = 0_usize;
    let mut preset = 0_usize;
    let mut note = String::from("Try RUN, then PULSE to inject a signal.");
    loop {
        let sw = screen_width();
        let sh = screen_height();
        let controls_top = sh - 216.0;
        let viewport = Rect::new(9.0, 96.0, sw - 18.0, (controls_top - 140.0).max(80.0));

        if playing {
            accumulator += get_frame_time().min(0.12);
            let period = 1.0 / speed as f32;
            let mut ticks = 0;
            while accumulator >= period && ticks < 4 {
                world.step();
                accumulator -= period;
                ticks += 1;
                dirty = true;
            }
            if ticks == 4 { accumulator = accumulator.min(period); }
        } else {
            accumulator = 0.0;
        }

        let mut buttons: Vec<Rect> = Vec::with_capacity(16);
        let side = 9.0;
        let gap = 5.0;
        let bw = (sw - 2.0 * side - 3.0 * gap) / 4.0;
        let bh = 43.0;
        for row in 0..4 {
            for col in 0..4 {
                buttons.push(Rect::new(side + col as f32 * (bw + gap),
                    controls_top + row as f32 * (bh + 6.0), bw, bh));
            }
        }

        let list = touches();
        if list.len() >= 2 {
            let span = list[0].position.distance(list[1].position);
            if let Some(prev) = previous_pinch {
                if prev > 5.0 {
                    zoom = (zoom * span / prev).clamp(0.45, 8.0);
                }
            }
            previous_pinch = Some(span);
            last_pointer = None;
        } else {
            previous_pinch = None;
        }
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            zoom = (zoom * (1.0 + wheel * 0.13).max(0.5)).clamp(0.45, 8.0);
        }

        let tap = if list.len() >= 2 { None } else { valid_tap() };
        if let Some(p) = tap {
            if let Some(i) = buttons.iter().position(|r| r.contains(p)) {
                match i {
                    0 => { playing = !playing; note = if playing { "Simulation running." } else { "Simulation paused." }.into(); }
                    1 => { world.step(); dirty = true; playing = false; note = "Stepped one generation.".into(); }
                    2 => { speed = match speed { 2 => 8, 8 => 24, _ => 2 }; note = format!("Speed: {} ticks/second", speed); }
                    3 => {
                        preset = (preset + 1) % 3;
                        world.preset(preset);
                        playing = false;
                        dirty = true;
                        center = vec2(64.0, 64.0);
                        zoom = 1.0;
                        note = match preset {
                            0 => "Straight-wire signal demo.",
                            1 => "Persistent loop demo. Run and watch.",
                            _ => "Empty sandbox. Build anything.",
                        }.into();
                    }
                    4 => { tool = Tool::Wire; note = "Drag to draw conductor.".into(); }
                    5 => { tool = Tool::Head; note = "Tap conductor to inject a pulse.".into(); }
                    6 => { tool = Tool::Tail; note = "Draw an electron tail.".into(); }
                    7 => { tool = Tool::Erase; note = "Drag to erase cells.".into(); }
                    8 => { tool = Tool::Pan; note = "Drag to move the view.".into(); }
                    9 => { zoom = (zoom / 1.7).max(0.45); }
                    10 => { zoom = (zoom * 1.7).min(8.0); }
                    11 => { world.clear(); playing = false; dirty = true; note = "Blank experiment board.".into(); }
                    12 => {
                        playing = false;
                        tool = Tool::Copy;
                        note = format!("Tap the center of a {}x{} region to save in slot {}.", STAMP_SIZE, STAMP_SIZE, slot + 1);
                    }
                    13 => {
                        if blueprints[slot].is_some() {
                            tool = Tool::Stamp;
                            note = format!("Tap to stamp blueprint in slot {}.", slot + 1);
                        } else { note = format!("Slot {} is empty. Use COPY first.", slot + 1); }
                    }
                    14 => {
                        if let Some(blueprint) = blueprints[slot].as_ref() {
                            let rotated = blueprint.rotated_clockwise();
                            save_blueprint(slot, &rotated);
                            blueprints[slot] = Some(rotated);
                            note = format!("Rotated slot {} clockwise.", slot + 1);
                        } else { note = "No blueprint to rotate.".into(); }
                    }
                    15 => {
                        slot = (slot + 1) % SLOTS;
                        note = format!("Slot {}: {}", slot + 1,
                            if blueprints[slot].is_some() { "saved blueprint" } else { "empty" });
                    }
                    _ => {}
                }
                last_pointer = None;
            } else if viewport.contains(p) {
                let source = view_range(center, zoom, viewport);
                let (x, y) = map_cell(p, viewport, source);
                match tool {
                    Tool::Copy => {
                        let blueprint = Blueprint::capture(&world,
                            x as isize - STAMP_SIZE as isize / 2,
                            y as isize - STAMP_SIZE as isize / 2,
                            STAMP_SIZE, STAMP_SIZE);
                        let n = blueprint.nonempty();
                        save_blueprint(slot, &blueprint);
                        blueprints[slot] = Some(blueprint);
                        tool = Tool::Stamp;
                        note = format!("Saved {} cells into slot {}. Tap to stamp.", n, slot + 1);
                    }
                    Tool::Stamp => {
                        if let Some(bp) = &blueprints[slot] {
                            bp.stamp(&mut world, x as isize, y as isize);
                            dirty = true;
                        }
                    }
                    _ => {}
                }
            }
        }

        if list.len() < 2 {
            let pointer = pointer_down().filter(|p| viewport.contains(*p));
            if let Some(current) = pointer {
                if let Some(last) = last_pointer {
                    if tool == Tool::Pan {
                        let source = view_range(center, zoom, viewport);
                        center.x -= (current.x - last.x) / viewport.w * source.w;
                        center.y -= (current.y - last.y) / viewport.h * source.h;
                    } else if let Some(material) = tool.state() {
                        let source = view_range(center, zoom, viewport);
                        let dist = current.distance(last);
                        let strokes = (dist / 4.0).ceil().clamp(1.0, 80.0) as usize;
                        for i in 0..=strokes {
                            let t = i as f32 / strokes as f32;
                            let p = last.lerp(current, t);
                            if viewport.contains(p) {
                                let (x, y) = map_cell(p, viewport, source);
                                world.set(x, y, material);
                                dirty = true;
                            }
                        }
                    }
                } else if let Some(material) = tool.state() {
                    let source = view_range(center, zoom, viewport);
                    let (x, y) = map_cell(current, viewport, source);
                    world.set(x, y, material);
                    dirty = true;
                }
            }
            last_pointer = pointer;
        }

        if dirty {
            refresh_texture(&mut image, &texture, &world);
            dirty = false;
        }

        clear_background(Color::from_rgba(11, 18, 30, 255));
        draw_text("GENESIS  /  INVENTION LAB", 12.0, 33.0,
            (sw / 21.0).clamp(17.0, 29.0), WHITE);
        let counts = world.counts();
        draw_text(&format!("Tick {}  |  Wire {}  |  Signals {}",
            world.tick, counts[1], counts[2]), 12.0, 59.0, 17.0, LIGHTGRAY);
        draw_text(&format!("{}  |  {} t/s  |  Zoom {:.1}x",
            tool.label(), speed, zoom), 12.0, 80.0, 15.0, Color::from_rgba(120, 210, 220, 255));

        let source = view_range(center, zoom, viewport);
        draw_texture_ex(&texture, viewport.x, viewport.y, WHITE,
            DrawTextureParams {
                source: Some(source),
                dest_size: Some(vec2(viewport.w, viewport.h)),
                ..Default::default()
            });
        draw_rectangle_lines(viewport.x, viewport.y, viewport.w, viewport.h, 1.0,
            Color::from_rgba(105, 135, 162, 255));

        // Show the fundamental cell boundaries when sufficiently zoomed in.
        if source.w <= 28.0 && source.h <= 44.0 {
            for gx in (source.x.ceil() as usize)..=((source.x + source.w).floor() as usize) {
                let x = viewport.x + (gx as f32 - source.x) * viewport.w / source.w;
                if x >= viewport.x && x <= viewport.x + viewport.w {
                    draw_line(x, viewport.y, x, viewport.y + viewport.h, 1.0,
                        Color::from_rgba(220, 228, 244, 40));
                }
            }
            for gy in (source.y.ceil() as usize)..=((source.y + source.h).floor() as usize) {
                let y = viewport.y + (gy as f32 - source.y) * viewport.h / source.h;
                if y >= viewport.y && y <= viewport.y + viewport.h {
                    draw_line(viewport.x, y, viewport.x + viewport.w, y, 1.0,
                        Color::from_rgba(220, 228, 244, 40));
                }
            }
        }

        draw_text(&note.chars().take(55).collect::<String>(),
            11.0, controls_top - 13.0, 14.0, LIGHTGRAY);
        let labels = [
            if playing { "PAUSE" } else { "RUN" }, "STEP", "SPEED", "DEMO",
            "WIRE", "PULSE", "TAIL", "ERASE",
            "PAN", "ZOOM -", "ZOOM +", "CLEAR",
            "COPY", "STAMP", "ROTATE", "SLOT",
        ];
        for (i, r) in buttons.iter().enumerate() {
            let selected = match i {
                0 => playing, 4 => tool == Tool::Wire, 5 => tool == Tool::Head,
                6 => tool == Tool::Tail, 7 => tool == Tool::Erase,
                8 => tool == Tool::Pan, 12 => tool == Tool::Copy,
                13 => tool == Tool::Stamp, 15 => blueprints[slot].is_some(),
                _ => false,
            };
            button(*r, labels[i], selected);
        }
        draw_text(&format!("Blueprint {}/4  |  {}",
            slot + 1, if blueprints[slot].is_some() { "SAVED" } else { "EMPTY" }),
            12.0, sh - 7.0, 11.0, LIGHTGRAY);
        next_frame().await;
    }
}
