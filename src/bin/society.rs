//! Genesis V0.3 graphical viewer — OpenFront-like legibility:
//! dots = agents, green pixels = resource stock, lines = actual sharing ties,
//! colors and labels = observer-detected clusters (not predefined factions).
use genesis::society::{Observer, World, SIDE};
use macroquad::prelude::*;

fn config() -> Conf {
    Conf {
        window_title: "Genesis — Emergent Societies".into(),
        window_width: 450,
        window_height: 800,
        high_dpi: true,
        ..Default::default()
    }
}

const PALETTE: [(u8, u8, u8); 9] = [
    (68, 180, 236), (248, 172, 92), (163, 128, 246),
    (242, 114, 145), (96, 207, 147), (240, 208, 92),
    (99, 189, 208), (220, 142, 189), (178, 215, 125)
];
fn group_color(id: u32) -> Color {
    let (r, g, b) = PALETTE[(id.saturating_sub(1) as usize) % PALETTE.len()];
    Color::from_rgba(r, g, b, 255)
}

fn point_to_screen(x: f32, y: f32, board: Rect, visible: Rect) -> Vec2 {
    vec2(
        board.x + (x - visible.x) / visible.w * board.w,
        board.y + (y - visible.y) / visible.h * board.h,
    )
}

fn visible_world(center: Vec2, zoom: f32) -> Rect {
    let span = (SIDE as f32 / zoom).clamp(9.0, SIDE as f32);
    let left = (center.x - span * 0.5).clamp(0.0, SIDE as f32 - span);
    let top = (center.y - span * 0.5).clamp(0.0, SIDE as f32 - span);
    Rect::new(left, top, span, span)
}

fn button(rect: Rect, text: &str, active: bool) {
    let bg = if active { Color::from_rgba(35, 100, 116, 255) }
        else { Color::from_rgba(33, 51, 68, 255) };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, bg);
    let size = (rect.w / (text.len() as f32 * 0.56 + 1.0)).clamp(11., 16.);
    let measure = measure_text(text, None, size as u16, 1.0);
    draw_text(text, rect.x + (rect.w - measure.width) * 0.5,
        rect.y + (rect.h + measure.height) * 0.5 - 2., size, WHITE);
}

fn update_food_texture(world: &World, image: &mut Image, texture: &Texture2D) {
    for y in 0..SIDE {
        for x in 0..SIDE {
            let tile = world.tiles[y * SIDE + x];
            let color = if tile.stock == 0 {
                Color::from_rgba(12, 20, 31, 255)
            } else {
                let fraction = tile.stock as f32 / tile.capacity.max(1) as f32;
                Color::new(0.10 + 0.12 * fraction, 0.28 + 0.36 * fraction,
                    0.22 + 0.09 * fraction, 1.0)
            };
            image.set_pixel(x as u32, y as u32, color);
        }
    }
    texture.update(image);
}

fn touch_started() -> Option<Vec2> {
    if let Some(touch) = touches().into_iter().find(|t| t.phase == TouchPhase::Started) {
        return Some(touch.position);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        let (x, y) = mouse_position();
        return Some(vec2(x, y));
    }
    None
}

fn active_pointer() -> Option<Vec2> {
    let contacts = touches();
    if let Some(touch) = contacts.first() {
        if touch.phase != TouchPhase::Ended && touch.phase != TouchPhase::Cancelled {
            return Some(touch.position);
        }
    }
    if is_mouse_button_down(MouseButton::Left) {
        let (x, y) = mouse_position();
        return Some(vec2(x, y));
    }
    None
}

fn ellipse_around_group(world: &World, members: &[usize]) -> (Vec2, f32) {
    let count = members.len() as f32;
    let center = members.iter().fold(vec2(0.0, 0.0), |sum, &i| {
        let a = world.agents[i];
        sum + vec2(a.x as f32 + 0.5, a.y as f32 + 0.5)
    }) / count.max(1.0);
    let radius = members.iter().map(|&i| {
        let a = world.agents[i];
        vec2(a.x as f32 + 0.5, a.y as f32 + 0.5).distance(center)
    }).fold(0.0_f32, f32::max);
    (center, (radius + 3.0).max(4.0))
}

#[macroquad::main(config)]
async fn main() {
    let mut seed = 73021_u64;
    let mut world = World::new(seed);
    let mut observer = Observer::default();
    let mut running = true;
    let mut speed = 8_usize;
    let mut accumulator = 0.0_f32;
    let mut zoom = 1.0_f32;
    let mut center = vec2(64.0, 64.0);
    let mut previous_pointer: Option<Vec2> = None;
    let mut previous_pinch: Option<f32> = None;
    let mut show_groups = true;
    let mut show_links = true;
    let mut show_food = true;
    let mut show_info = false;
    let mut selected: Option<usize> = None;
    let mut image = Image::gen_image_color(SIDE as u16, SIDE as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    let mut last_food_tick = u64::MAX;

    loop {
        let sw = screen_width();
        let sh = screen_height();
        let controls_y = sh - 171.0;
        let board_side = (sw - 22.0).min((controls_y - 138.0).max(90.0));
        let board = Rect::new((sw - board_side) * 0.5, 105.0, board_side, board_side);
        let buttons_gap = 5.0;
        let button_width = (sw - 18.0 - 3.0 * buttons_gap) / 4.0;
        let mut buttons = Vec::with_capacity(12);
        for row in 0..3 {
            for col in 0..4 {
                buttons.push(Rect::new(9.0 + col as f32 * (button_width + buttons_gap),
                    controls_y + row as f32 * 49.0, button_width, 43.0));
            }
        }

        if running {
            accumulator += get_frame_time().min(0.16);
            let interval = 1.0 / speed as f32;
            let mut steps = 0;
            while accumulator >= interval && steps < 4 {
                world.step();
                accumulator -= interval;
                steps += 1;
                if world.tick % 24 == 0 { observer.observe(&world); }
            }
            if steps == 4 { accumulator = accumulator.min(interval); }
        } else { accumulator = 0.0; }

        let contacts = touches();
        if contacts.len() >= 2 {
            let distance = contacts[0].position.distance(contacts[1].position);
            if let Some(previous) = previous_pinch {
                if previous > 5.0 { zoom = (zoom * distance / previous).clamp(1.0, 14.0); }
            }
            previous_pinch = Some(distance);
            previous_pointer = None;
        } else {
            previous_pinch = None;
        }
        let wheel = mouse_wheel().1;
        if wheel.abs() > 0.01 {
            zoom = (zoom * (1.0 + wheel * 0.13).max(0.5)).clamp(1.0, 14.0);
        }

        let pressed = if contacts.len() >= 2 { None } else { touch_started() };
        if let Some(p) = pressed {
            if let Some(i) = buttons.iter().position(|rect| rect.contains(p)) {
                match i {
                    0 => running = !running,
                    1 => {
                        world.step();
                        if world.tick % 24 == 0 { observer.observe(&world); }
                        running = false;
                    }
                    2 => speed = match speed { 2 => 8, 8 => 24, _ => 2 },
                    3 => world.rules.sharing = !world.rules.sharing,
                    4 => show_groups = !show_groups,
                    5 => show_links = !show_links,
                    6 => show_food = !show_food,
                    7 => {
                        seed = seed.wrapping_add(1);
                        world = World::new(seed);
                        observer = Observer::default();
                        last_food_tick = u64::MAX;
                        selected = None;
                        running = true;
                        accumulator = 0.0;
                    }
                    8 => zoom = (zoom / 1.6).max(1.0),
                    9 => zoom = (zoom * 1.6).min(14.0),
                    10 => { zoom = 1.0; center = vec2(64.0, 64.0); }
                    11 => show_info = !show_info,
                    _ => {}
                }
                previous_pointer = None;
            } else if board.contains(p) {
                let visible = visible_world(center, zoom);
                let mut nearest: Option<usize> = None;
                let mut best = 11.0_f32;
                for (i, agent) in world.agents.iter().enumerate() {
                    let position = point_to_screen(
                        agent.x as f32 + 0.5, agent.y as f32 + 0.5, board, visible);
                    let distance = position.distance(p);
                    if distance < best {
                        best = distance;
                        nearest = Some(i);
                    }
                }
                selected = nearest;
            }
        }
        if contacts.len() < 2 {
            let current = active_pointer().filter(|p| board.contains(*p));
            if let (Some(point), Some(previous)) = (current, previous_pointer) {
                let visible = visible_world(center, zoom);
                center.x -= (point.x - previous.x) * visible.w / board.w;
                center.y -= (point.y - previous.y) * visible.h / board.h;
            }
            previous_pointer = current;
        }

        let visible = visible_world(center, zoom);
        if last_food_tick == u64::MAX || (show_food && world.tick >= last_food_tick.saturating_add(12)) {
            update_food_texture(&world, &mut image, &texture);
            last_food_tick = world.tick;
        }

        clear_background(Color::from_rgba(11, 18, 29, 255));
        draw_text("GENESIS", 12.0, 34.0, 32.0, WHITE);
        draw_text("EMERGENT SOCIETIES", 13.0, 54.0, 15.0, LIGHTGRAY);
        draw_text(&format!("Tick {}   Alive {}/{}   Groups {}",
            world.tick, world.alive_count(), world.agents.len(), observer.stable_groups()),
            13.0, 77.0, 17.0, Color::from_rgba(191, 211, 228, 255));
        draw_text(&format!("Shared {} energy   Seed {}", world.cumulative_transferred, seed),
            13.0, 95.0, 13.0, Color::from_rgba(143, 165, 180, 255));

        draw_rectangle(board.x, board.y, board.w, board.h,
            Color::from_rgba(15, 25, 39, 255));
        if show_food {
            draw_texture_ex(&texture, board.x, board.y, WHITE, DrawTextureParams {
                source: Some(visible),
                dest_size: Some(vec2(board.w, board.h)),
                ..Default::default()
            });
        }

        let membership = observer.membership(world.agents.len());
        if show_groups {
            for group in observer.groups.iter().filter(|g| g.visible()) {
                let (center_world, radius_world) = ellipse_around_group(&world, &group.members);
                let point = point_to_screen(center_world.x, center_world.y, board, visible);
                let radius = radius_world / visible.w * board.w;
                let col = group_color(group.id);
                if radius < board.w * 0.8 {
                    draw_circle(point.x, point.y, radius,
                        Color::new(col.r, col.g, col.b, 0.045));
                    for part in (0..36).step_by(2) {
                        let a = part as f32 / 36.0 * std::f32::consts::TAU;
                        let b = (part + 1) as f32 / 36.0 * std::f32::consts::TAU;
                        draw_line(point.x + radius * a.cos(), point.y + radius * a.sin(),
                            point.x + radius * b.cos(), point.y + radius * b.sin(),
                            1.5, Color::new(col.r, col.g, col.b, 0.53));
                    }
                }
                if board.contains(point) {
                    draw_text(&format!("GROUP {}", group.id),
                        (point.x - 31.0).max(board.x + 3.0),
                        (point.y - radius - 5.0).max(board.y + 15.0),
                        13.0, col);
                }
            }
        }

        if show_links {
            for i in 0..world.agents.len() {
                let ai = world.agents[i];
                if !ai.alive { continue; }
                for j in (i + 1)..world.agents.len() {
                    let aj = world.agents[j];
                    if !aj.alive || world.tie(i, j) < 28 { continue; }
                    // Avoid huge lines caused by periodic wrapping of the world.
                    if (ai.x - aj.x).abs() > 16 || (ai.y - aj.y).abs() > 16 { continue; }
                    let a = point_to_screen(ai.x as f32 + 0.5, ai.y as f32 + 0.5, board, visible);
                    let b = point_to_screen(aj.x as f32 + 0.5, aj.y as f32 + 0.5, board, visible);
                    if board.contains(a) && board.contains(b) {
                        let fade = (world.tie(i, j) as f32 / 255.0).clamp(0.12, 0.40);
                        draw_line(a.x, a.y, b.x, b.y, 1.2,
                            Color::new(0.61, 0.73, 0.85, fade));
                    }
                }
            }
        }

        let dot = (board.w / visible.w * 0.7).clamp(2.6, 5.6);
        for (i, agent) in world.agents.iter().enumerate() {
            let screen = point_to_screen(agent.x as f32 + 0.5,
                agent.y as f32 + 0.5, board, visible);
            if !board.contains(screen) { continue; }
            let c = if !agent.alive {
                Color::from_rgba(76, 86, 105, 255)
            } else if show_groups {
                membership[i].map(group_color).unwrap_or(
                    Color::from_rgba(224, 232, 241, 255))
            } else {
                Color::from_rgba(224, 232, 241, 255)
            };
            draw_circle(screen.x, screen.y, if agent.alive { dot } else { dot * 0.65 }, c);
            if selected == Some(i) {
                draw_circle_lines(screen.x, screen.y, dot + 4.0, 2.0, YELLOW);
            }
        }
        draw_rectangle_lines(board.x, board.y, board.w, board.h, 1.0,
            Color::from_rgba(78, 104, 129, 255));

        let hint_y = board.y + board.h + 18.0;
        if let Some(id) = selected {
            let a = world.agents[id];
            let group = membership[id].map(|id| format!("Group {}", id))
                .unwrap_or_else(|| "No detected group".into());
            draw_text(&format!("Dot {}: energy {}  |  {}", id + 1, a.energy, group),
                12.0, hint_y, 13.0, LIGHTGRAY);
        } else {
            draw_text("Dots: agents   •   Lines: sharing   •   Green: resources",
                12.0, hint_y, 13.0, LIGHTGRAY);
        }

        let labels = [
            if running { "PAUSE" } else { "RUN" }, "STEP", "SPEED",
            if world.rules.sharing { "SHARE ON" } else { "SHARE OFF" },
            if show_groups { "GROUPS ON" } else { "GROUPS OFF" },
            if show_links { "LINES ON" } else { "LINES OFF" },
            if show_food { "FOOD ON" } else { "FOOD OFF" }, "NEW WORLD",
            "ZOOM -", "ZOOM +", "CENTER", "INFO",
        ];
        for (i, rect) in buttons.iter().enumerate() {
            button(*rect, labels[i],
                match i {
                    0 => running, 3 => world.rules.sharing, 4 => show_groups,
                    5 => show_links, 6 => show_food, 11 => show_info,
                    _ => false,
                });
        }
        if show_info {
            let overlay = Rect::new(board.x + 12.0, board.y + 12.0,
                board.w - 24.0, 112.0);
            draw_rectangle(overlay.x, overlay.y, overlay.w, overlay.h,
                Color::from_rgba(17, 32, 49, 245));
            for (row, text) in [
                "100 independent agents; no starting factions.",
                "Agents move, eat, share surpluses, remember food.",
                "Colors are read-only observations of repeated sharing.",
                "Turn SHARE OFF to compare identical-world rules.",
            ].iter().enumerate() {
                draw_text(text, overlay.x + 8.0, overlay.y + 23.0 + row as f32 * 24.0,
                    (overlay.w / 33.0).clamp(10.0, 14.0), WHITE);
            }
        }
        draw_text(&format!("{} ticks/s • pinch to zoom • drag to pan • tap a dot",
            speed), 13.0, sh - 12.0, 12.0, LIGHTGRAY);
        next_frame().await;
    }
}
