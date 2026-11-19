//! Genesis V0.4 graphical viewer — autonomous society, player-controlled squad:
//! dots = agents, green pixels = resource stock, lines = recorded social ties,
//! colors and labels = observer-detected clusters (not predefined factions).
use genesis::society::{Observer, World, SIDE};
use genesis::military::Order;
use macroquad::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pending {
    Recruit,
    Move,
    Hold,
}

fn config() -> Conf {
    Conf {
        window_title: "Genesis — First Army".into(),
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
    let size = (rect.w / (text.len() as f32 * 0.56 + 1.0)).clamp(13., 19.);
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

/// A purely player-facing objective. It NEVER modifies the simulation,
/// agent decisions, or group observer labels.
fn hold_progress(world: &World, previous: u32) -> u32 {
    let Some(squad) = &world.squad else { return 0; };
    if squad.order != Order::Hold { return 0; }
    let defenders = squad.members.iter().filter(|&&id| {
        let a = &world.agents[id];
        a.alive && World::distance((a.x, a.y), squad.target) <= 3
    }).count();
    if defenders >= 3 { previous.saturating_add(1).min(20) } else { 0 }
}

#[macroquad::main(config)]
async fn main() {
    let mut seed = 73021_u64;
    let mut world = World::new(seed);
    let mut observer = Observer::default();
    // Begin paused: the player can understand the board before agents die.
    let mut running = false;
    let mut speed = 2_usize;
    let mut accumulator = 0.0_f32;
    let mut zoom = 1.0_f32;
    let mut center = vec2(64.0, 64.0);
    let mut previous_pointer: Option<Vec2> = None;
    let mut previous_pinch: Option<f32> = None;
    let mut show_groups = true;
    let mut show_links = true;
    let mut show_food = true;
    let mut pending: Option<Pending> = None;
    let mut notice = String::from("Tap the map to recruit eight nearby civilians.");
    let mut options = false;
    let mut held_ticks = 0_u32;
    let mut selected: Option<usize> = None;
    let mut image = Image::gen_image_color(SIDE as u16, SIDE as u16, BLACK);
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Nearest);
    let mut last_food_tick = u64::MAX;

    loop {
        let sw = screen_width();
        let sh = screen_height();
        // Big 3-column touch controls. On wide screens, the controls move
        // to a right rail; on phones, they sit below the map.
        let wide = sw > sh * 1.25;
        let panel_w = if wide { (sw * 0.35).clamp(285.0, 370.0) } else { sw };
        let panel_x = if wide { sw - panel_w } else { 0.0 };
        let controls_y = if wide { 172.0 } else { sh - 206.0 };
        let board_w = if wide { panel_x - 22.0 } else { sw - 22.0 };
        let board_h = if wide { sh - 112.0 } else { controls_y - 148.0 };
        let board_side = board_w.min(board_h).max(86.0);
        let board = Rect::new(
            if wide { 12.0 } else { (sw - board_side) * 0.5 },
            87.0, board_side, board_side);
        let gap = 7.0;
        let button_width = (panel_w - 24.0 - 2.0 * gap) / 3.0;
        let button_h = 53.0;
        let mut buttons = Vec::with_capacity(9);
        for row in 0..3 {
            for col in 0..3 {
                buttons.push(Rect::new(panel_x + 12.0 + col as f32 * (button_width + gap),
                    controls_y + row as f32 * (button_h + gap), button_width, button_h));
            }
        }

        if running {
            accumulator += get_frame_time().min(0.16);
            let interval = 1.0 / speed as f32;
            let mut steps = 0;
            while accumulator >= interval && steps < 4 {
                world.step();
                held_ticks = hold_progress(&world, held_ticks);
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
                if !options {
                    match i {
                        0 => {
                            running = !running;
                            notice = if running { "Simulation running." }
                                else { "Paused. Choose a command or tap the map." }.into();
                        }
                        1 => {
                            pending = Some(Pending::Recruit);
                            notice = "RECRUIT: tap near civilians on the map.".into();
                        }
                        2 => {
                            seed = seed.wrapping_add(1);
                            world = World::new(seed);
                            observer = Observer::default();
                            last_food_tick = u64::MAX;
                            selected = None;
                            pending = None;
                            running = false;
                            held_ticks = 0;
                            accumulator = 0.0;
                            center = vec2(64.0, 64.0);
                            zoom = 1.0;
                            notice = "New world. Tap the map to recruit soldiers.".into();
                        }
                        3 => {
                            if world.squad.is_some() {
                                pending = Some(Pending::Move);
                                notice = "MOVE: now tap a destination on the map.".into();
                            } else { pending = Some(Pending::Recruit);
                                notice = "First recruit: tap near civilians.".into(); }
                        }
                        4 => {
                            if world.squad.is_some() {
                                pending = Some(Pending::Hold);
                                notice = "HOLD: tap a green area to defend it.".into();
                            } else { pending = Some(Pending::Recruit);
                                notice = "First recruit: tap near civilians.".into(); }
                        }
                        5 => {
                            if world.issue_order(Order::Retreat, (0, 0)) {
                                held_ticks = 0;
                                pending = None;
                                running = true;
                                notice = "RETREAT: squad returning to recruitment site.".into();
                            } else { notice = "You need to recruit soldiers first.".into(); }
                        }
                        6 => {
                            world.disband();
                            held_ticks = 0;
                            pending = None;
                            notice = "Soldiers disbanded; they resume civilian life.".into();
                        }
                        7 => zoom = (zoom * 1.6).min(14.0),
                        8 => {
                            options = true;
                            notice = "Options: step, speed, layers and camera.".into();
                        }
                        _ => {}
                    }
                } else {
                    match i {
                        0 => { options = false; notice = "Tap the map to recruit or command.".into(); }
                        1 => {
                            world.step();
                            held_ticks = hold_progress(&world, held_ticks);
                            if world.tick % 24 == 0 { observer.observe(&world); }
                            running = false;
                            notice = "Advanced one simulation tick.".into();
                        }
                        2 => { speed = match speed { 2 => 8, 8 => 24, _ => 2 };
                            notice = format!("Simulation speed: {speed} ticks/sec."); }
                        3 => show_food = !show_food,
                        4 => show_groups = !show_groups,
                        5 => show_links = !show_links,
                        6 => world.rules.sharing = !world.rules.sharing,
                        7 => zoom = (zoom / 1.6).max(1.0),
                        8 => { center = vec2(64.0, 64.0); zoom = 1.0; }
                        _ => {}
                    }
                }
                previous_pointer = None;
            } else if board.contains(p) {
                let visible = visible_world(center, zoom);
                let world_x = (visible.x + (p.x - board.x) / board.w * visible.w).floor() as i16;
                let world_y = (visible.y + (p.y - board.y) / board.h * visible.h).floor() as i16;
                let target = (world_x.clamp(0, SIDE as i16 - 1),
                    world_y.clamp(0, SIDE as i16 - 1));
                // One-tap onboarding: without a squad, any tap recruits;
                // with a squad, a simple tap issues MOVE without another menu.
                let action = pending.take().or_else(|| if world.squad.is_none() {
                    Some(Pending::Recruit)
                } else {
                    Some(Pending::Move)
                });
                if let Some(action) = action {
                    match action {
                        Pending::Recruit => {
                            let count = world.recruit_nearest(target, 8);
                            held_ticks = 0;
                            if count > 0 {
                                running = true;
                                notice = format!("{} recruited. Tap the map to MOVE, or choose HOLD.", count);
                            } else { notice = "No living agents available for recruitment.".into(); }
                        }
                        Pending::Move => {
                            if world.issue_order(Order::Move, target) {
                                held_ticks = 0;
                                running = true;
                                notice = format!("MOVE to ({}, {}). Watch cyan soldiers.", target.0, target.1);
                            }
                        }
                        Pending::Hold => {
                            if world.issue_order(Order::Hold, target) {
                                held_ticks = 0;
                                running = true;
                                notice = format!("HOLD near ({}, {}). Outnumbered guards can lose.", target.0, target.1);
                            }
                        }
                    }
                }

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
        draw_text("GENESIS", 12.0, 34.0, 30.0, WHITE);
        draw_text("COMMAND THE ARMY. THE WORLD RUNS ITSELF.", 13.0, 55.0,
            if sw < 410.0 { 12.0 } else { 15.0 }, LIGHTGRAY);
        draw_text(&format!("Tick {}   Alive {}   Groups {}   Army {}",
            world.tick, world.alive_count(), observer.stable_groups(), world.soldier_count()),
            13.0, 77.0, 17.0, Color::from_rgba(191, 211, 228, 255));
        draw_text(&format!("Denied {}  Overrun {}  World {}",
            world.cumulative_access_denied, world.cumulative_access_overrun, seed),
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

        // Military overlays are player-issued commands, not emergent kingdoms.
        if let Some(squad) = &world.squad {
            let dest = squad.destination();
            let target_p = point_to_screen(
                dest.0 as f32 + 0.5, dest.1 as f32 + 0.5, board, visible);
            if board.contains(target_p) {
                let c = Color::from_rgba(86, 207, 248, 255);
                draw_circle_lines(target_p.x, target_p.y,
                    (4.0 * board.w / visible.w).clamp(9.0, 24.0), 1.7, c);
                draw_text(squad.order.name(),
                    (target_p.x + 12.0).min(board.x + board.w - 72.0),
                    (target_p.y - 10.0).max(board.y + 14.0), 13.0, c);
            }
            let mut x = 0.0;
            let mut y = 0.0;
            let mut total = 0.0;
            for &id in &squad.members {
                if world.agents[id].alive {
                    x += world.agents[id].x as f32 + 0.5;
                    y += world.agents[id].y as f32 + 0.5;
                    total += 1.0;
                }
            }
            if total > 0.0 {
                let army_p = point_to_screen(x / total, y / total, board, visible);
                if board.contains(army_p) && board.contains(target_p) {
                    draw_line(army_p.x, army_p.y, target_p.x, target_p.y,
                        1.7, Color::new(0.37, 0.77, 0.93, 0.6));
                }
            }
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
            } else if world.soldier(i) {
                Color::from_rgba(90, 222, 250, 255)
            } else if show_groups {
                membership[i].map(group_color).unwrap_or(
                    Color::from_rgba(224, 232, 241, 255))
            } else {
                Color::from_rgba(224, 232, 241, 255)
            };
            draw_circle(screen.x, screen.y, if agent.alive { dot } else { dot * 0.65 }, c);
            if world.soldier(i) && agent.alive {
                draw_circle_lines(screen.x, screen.y, dot + 2.0, 1.0, c);
            }
            if selected == Some(i) {
                draw_circle_lines(screen.x, screen.y, dot + 4.0, 2.0, YELLOW);
            }
        }
        draw_rectangle_lines(board.x, board.y, board.w, board.h, 1.0,
            Color::from_rgba(78, 104, 129, 255));

        // Two lines of immediately actionable guidance, on every device.
        // No technical terminology and no unreadable 10pt footer instruction.
        let hint_x = if wide { panel_x + 13.0 } else { 12.0 };
        let hint_y = if wide { 100.0 } else { board.y + board.h + 20.0 };
        let hint_width = if wide { panel_w - 25.0 } else { sw - 25.0 };
        let hint_bg_h = if wide { 60.0 } else { 67.0 };
        draw_rectangle(hint_x - 4.0, hint_y - 16.0, hint_width, hint_bg_h,
            Color::from_rgba(29, 49, 66, 255));
        let instruction = if world.squad.is_none() {
            "1. TAP THE MAP TO RECRUIT 8 PEOPLE"
        } else if pending == Some(Pending::Recruit) {
            "TAP NEAR PEOPLE TO RECRUIT AGAIN"
        } else if pending == Some(Pending::Move) {
            "TAP THE MAP TO CHOOSE WHERE TO MOVE"
        } else if pending == Some(Pending::Hold) {
            "TAP A GREEN AREA TO HOLD IT"
        } else {
            "2. TAP MAP TO MOVE. OR PRESS HOLD."
        };
        let instruction_size = (hint_width / (instruction.len() as f32 * 0.58))
            .clamp(11.0, 17.0);
        draw_text(instruction, hint_x + 5.0, hint_y + 5.0,
            instruction_size, Color::from_rgba(139, 224, 244, 255));
        let sentence = if let Some(id) = selected {
            let a = world.agents[id];
            format!("Person {}: {} energy, {}",
                id + 1, a.energy, if world.soldier(id) { "soldier" } else { "civilian" })
        } else if let Some(squad) = &world.squad {
            format!("Soldiers: {}  Order: {}  Hold: {}/20",
                world.soldier_count(), squad.order.name(), held_ticks)
        } else {
            "White dots: people. Green: food. Cyan: your army.".into()
        };
        draw_text(&sentence, hint_x + 5.0, hint_y + 26.0,
            (hint_width / (sentence.len() as f32 * 0.57)).clamp(11., 15.),
            Color::from_rgba(216, 227, 240, 255));

        // Fill spare portrait space with a real objective and army health,
        // but never overlap the map or the phone's touch controls.
        if !wide {
            let panel_top = hint_y + hint_bg_h - 9.0;
            let available = controls_y - panel_top - 8.0;
            if available >= 52.0 {
                let panel_h = available.min(88.0);
                draw_rectangle(8.0, panel_top, sw - 16.0, panel_h,
                    Color::from_rgba(18, 32, 47, 255));
                let header = if held_ticks >= 20 {
                    "SITE SECURED - OBJECTIVE COMPLETE"
                } else if world.squad.is_some() {
                    "OBJECTIVE: HOLD A GREEN SITE FOR 20 TICKS"
                } else {
                    "YOUR FIRST MISSION: RECRUIT AND HOLD"
                };
                draw_text(header, 17.0, panel_top + 20.0,
                    ((sw - 35.0) / (header.len() as f32 * 0.59)).clamp(10.0, 16.0),
                    Color::from_rgba(157, 221, 176, 255));
                let avg_energy = if let Some(squad) = &world.squad {
                    let living: Vec<_> = squad.members.iter()
                        .map(|&id| &world.agents[id])
                        .filter(|a| a.alive).collect();
                    if living.is_empty() { 0 }
                    else { living.iter().map(|a| a.energy as u32).sum::<u32>()
                        / living.len() as u32 }
                } else { 0 };
                draw_text(&format!("Hold {}/20   Soldiers {}   Avg energy {}",
                    held_ticks, world.soldier_count(), avg_energy),
                    17.0, panel_top + 39.0, 13.0, LIGHTGRAY);
                if panel_h >= 58.0 {
                    let bar_y = panel_top + 46.0;
                    draw_rectangle(17.0, bar_y, sw - 34.0, 7.0,
                        Color::from_rgba(40, 56, 70, 255));
                    draw_rectangle(17.0, bar_y, (sw - 34.0) * held_ticks as f32 / 20.0,
                        7.0, Color::from_rgba(105, 212, 145, 255));
                }
            }
        }

        let labels = if options {
            [
                "BACK", "STEP", "SPEED",
                if show_food { "FOOD ON" } else { "FOOD OFF" },
                if show_groups { "GROUPS ON" } else { "GROUPS OFF" },
                if show_links { "LINES ON" } else { "LINES OFF" },
                if world.rules.sharing { "SHARE ON" } else { "SHARE OFF" },
                "ZOOM -", "CENTER",
            ]
        } else {
            [
                if running { "PAUSE" } else { "PLAY" }, "RECRUIT", "NEW WORLD",
                "MOVE", "HOLD", "RETREAT",
                "DISBAND", "ZOOM +", "OPTIONS",
            ]
        };
        for (i, rect) in buttons.iter().enumerate() {
            button(*rect, labels[i], if options {
                match i {
                    3 => show_food, 4 => show_groups, 5 => show_links,
                    6 => world.rules.sharing, _ => false,
                }
            } else {
                match i {
                    0 => running,
                    1 => pending == Some(Pending::Recruit),
                    3 => pending == Some(Pending::Move),
                    4 => pending == Some(Pending::Hold),
                    _ => false,
                }
            });
        }
        let footer = if wide { controls_y + 195.0 } else { sh - 9.0 };
        let bottom_x = if wide { panel_x + 12.0 } else { 12.0 };
        let display = notice.chars().take(56).collect::<String>();
        draw_text(&display, bottom_x, footer, 12.0, LIGHTGRAY);
        next_frame().await;
    }
}
