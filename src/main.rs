use macroquad::prelude::*;
const SIZE: usize = 8;
const COST: i32 = 10;

fn config() -> Conf {
    Conf { window_title: "Pocket Kingdom".into(), window_width: 480, window_height: 760, high_dpi: true, ..Default::default() }
}

#[macroquad::main(config)]
async fn main() {
    let mut owned = [[false; SIZE]; SIZE];
    owned[3][3] = true;
    let (mut gold, mut turn) = (50, 1);
    let mut note = String::from("Tap land to buy it for 10 gold");
    loop {
        let (w, h) = (screen_width(), screen_height());
        clear_background(Color::from_rgba(15, 23, 42, 255));
        draw_text("POCKET KINGDOM", 18., 50., (w / 14.).clamp(22., 40.), GOLD);
        draw_text(&format!("TURN {}     GOLD {}", turn, gold), 18., 86., 25., WHITE);
        let board = (w - 28.).min((h - 260.).max(160.));
        let cell = board / SIZE as f32;
        let left = (w - board) / 2.;
        let top = 112.;
        for y in 0..SIZE {
            for x in 0..SIZE {
                let c = if owned[y][x] { GREEN } else if (x + y) % 2 == 0 { Color::from_rgba(55,76,104,255) } else { Color::from_rgba(64,87,117,255) };
                draw_rectangle(left + x as f32 * cell + 1., top + y as f32 * cell + 1., cell - 2., cell - 2., c);
            }
        }
        let by = top + board + 22.;
        draw_rectangle(left, by, board, 58., Color::from_rgba(38,112,192,255));
        let label = "NEXT TURN  (+5 PER TILE)";
        let fs = (w / 25.).clamp(14., 23.);
        let dimensions = measure_text(label, None, fs as u16, 1.);
        draw_text(label, (w - dimensions.width) / 2., by + 37., fs, WHITE);
        draw_text(&note, 16., (by + 96.).min(h - 28.), (w / 30.).clamp(12., 19.), LIGHTGRAY);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if mx >= left && mx < left + board && my >= top && my < top + board {
                let x = ((mx - left) / cell) as usize;
                let y = ((my - top) / cell) as usize;
                if owned[y][x] { note = "Already owned".into(); }
                else if gold < COST { note = "Need more gold".into(); }
                else { owned[y][x] = true; gold -= COST; note = format!("Bought land {}, {}", x + 1, y + 1); }
            } else if mx >= left && mx < left + board && my >= by && my < by + 58. {
                let n = owned.iter().flatten().filter(|&&v| v).count() as i32;
                gold += n * 5;
                turn += 1;
                note = format!("Earned {} gold", n * 5);
            }
        }
        next_frame().await;
    }
}
