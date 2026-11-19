//! Genesis Frontier — readable competitive strategy, live Rust engine.
//! Play against three computer-controlled kingdoms. Real online multiplayer
//! remains a later networking milestone.
use genesis::frontier::{World, Terrain, Order, WIDTH, HEIGHT, FACTIONS, NEUTRAL};
use macroquad::prelude::*;
use std::collections::HashMap;

fn config() -> Conf {
    Conf {
        window_title: "Genesis — Frontier".into(),
        window_width: 450,
        window_height: 820,
        high_dpi: false, // Touch positions match CSS pixels on Android.
        ..Default::default()
    }
}

fn kingdom_color(id:u8)->Color {
    match id {
        0=>Color::from_rgba(71,174,229,255),
        1=>Color::from_rgba(231,151,89,255),
        2=>Color::from_rgba(112,201,136,255),
        3=>Color::from_rgba(173,133,233,255),
        _=>Color::from_rgba(102,118,136,255),
    }
}

fn terrain_color(terrain:Terrain,winter:bool)->Color {
    let (r,g,b)=match terrain {
        Terrain::Sea=>(28.,66.,90.),
        Terrain::Plain=>(68.,104.,83.),
        Terrain::Farm=>(98.,142.,86.),
        Terrain::Forest=>(42.,91.,70.),
        Terrain::Mountain=>(110.,119.,121.),
    };
    let c=Color::new(r/255.,g/255.,b/255.,1.);
    if winter && terrain!=Terrain::Sea {
        Color::new((c.r*0.45+0.45).min(1.),(c.g*0.45+0.48).min(1.),
            (c.b*0.45+0.48).min(1.),1.)
    } else {c}
}

fn blend(a:Color,b:Color,alpha:f32)->Color {
    Color::new(a.r*(1.-alpha)+b.r*alpha,
        a.g*(1.-alpha)+b.g*alpha,a.b*(1.-alpha)+b.b*alpha,1.)
}

fn refresh(world:&World,image:&mut Image,texture:&Texture2D) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile=world.tiles[y*WIDTH+x];
            let base=terrain_color(tile.terrain,world.winter());
            let color=if tile.owner==NEUTRAL || tile.terrain==Terrain::Sea {base}
                else {blend(base,kingdom_color(tile.owner),
                    if tile.terrain==Terrain::Mountain {0.35} else {0.70})};
            image.set_pixel(x as u32,y as u32,color);
        }
    }
    texture.update(image);
}

fn draw_button(rect:Rect,label:&str,active:bool) {
    draw_rectangle(rect.x,rect.y,rect.w,rect.h,
        if active {Color::from_rgba(37,119,141,255)}
        else {Color::from_rgba(33,51,70,255)});
    draw_rectangle_lines(rect.x,rect.y,rect.w,rect.h,1.,
        if active {Color::from_rgba(107,208,236,255)}
        else {Color::from_rgba(63,85,108,255)});
    let fs=(rect.w/(label.len() as f32*0.63+1.)).clamp(11.,17.);
    let t=measure_text(label,None,fs as u16,1.);
    draw_text(label,rect.x+(rect.w-t.width)*0.5,
        rect.y+(rect.h+t.height)*0.5-2.,fs,WHITE);
}

/// Map gestures are intentionally deferred until release, so a drag
/// never accidentally issues a move/attack order.
const DRAG_THRESHOLD_PX: f32 = 9.0;

#[derive(Clone, Copy)]
struct MapGesture {
    start: Vec2,
    last: Vec2,
    dragged: bool,
}

fn clamp_center(center: &mut Vec2, zoom: f32) {
    let span_x = (WIDTH as f32 / zoom).clamp(10.0, WIDTH as f32);
    let span_y = (HEIGHT as f32 / zoom).clamp(8.0, HEIGHT as f32);
    center.x = center.x.clamp(span_x * 0.5, WIDTH as f32 - span_x * 0.5);
    center.y = center.y.clamp(span_y * 0.5, HEIGHT as f32 - span_y * 0.5);
}

fn pan_by_pixels(center: &mut Vec2, zoom: f32, board: Rect, delta: Vec2) {
    let view = view_rect(*center, zoom);
    center.x -= delta.x * view.w / board.w;
    center.y -= delta.y * view.h / board.h;
    clamp_center(center, zoom);
}

/// Keep the terrain below the finger or mouse fixed while changing zoom.
fn zoom_at(center: &mut Vec2, zoom: &mut f32, scale: f32, anchor: Vec2, board: Rect) {
    let before = view_rect(*center, *zoom);
    let point_before = vec2(
        before.x + ((anchor.x - board.x) / board.w) * before.w,
        before.y + ((anchor.y - board.y) / board.h) * before.h,
    );
    *zoom = (*zoom * scale).clamp(1.0, 5.0);
    let after = view_rect(*center, *zoom);
    let point_after = vec2(
        after.x + ((anchor.x - board.x) / board.w) * after.w,
        after.y + ((anchor.y - board.y) / board.h) * after.h,
    );
    *center += point_before - point_after;
    clamp_center(center, *zoom);
}

fn screen_pos_from_world(pos: Vec2, board: Rect, view: Rect) -> Vec2 {
    vec2(board.x + (pos.x + 0.5 - view.x) / view.w * board.w,
         board.y + (pos.y + 0.5 - view.y) / view.h * board.h)
}

fn cell_to_screen(pos:(i16,i16),board:Rect,view:Rect)->Vec2 {
    vec2(board.x+(pos.0 as f32+0.5-view.x)/view.w*board.w,
        board.y+(pos.1 as f32+0.5-view.y)/view.h*board.h)
}

fn view_rect(center:Vec2,zoom:f32)->Rect {
    let w=(WIDTH as f32/zoom).clamp(10.,WIDTH as f32);
    let h=(HEIGHT as f32/zoom).clamp(8.,HEIGHT as f32);
    Rect::new((center.x-w*0.5).clamp(0.,WIDTH as f32-w),
        (center.y-h*0.5).clamp(0.,HEIGHT as f32-h),w,h)
}

fn map_cell(pos:Vec2,board:Rect,view:Rect)->(i16,i16) {
    let x=(view.x+((pos.x-board.x)/board.w)*view.w).floor()
        .clamp(0.,(WIDTH-1) as f32) as i16;
    let y=(view.y+((pos.y-board.y)/board.h)*view.h).floor()
        .clamp(0.,(HEIGHT-1) as f32) as i16;
    (x,y)
}

fn player_army<'a>(world:&'a World,selected:u32)->Option<&'a genesis::frontier::Army>{
    world.armies.iter().find(|a|a.id==selected && a.owner==0 && a.troops>0)
}

fn current_army(world:&World,selected:&mut u32) {
    if player_army(world,*selected).is_none() {
        *selected=world.armies.iter().find(|a|a.owner==0 && a.troops>0)
            .map_or(0,|a|a.id);
    }
}

#[macroquad::main(config)]
async fn main() {
    let mut seed=77001_u64;
    let mut world=World::new(seed);
    let mut selected=world.armies[0].id;
    let mut running=true;
    let mut speed=8_usize;
    let mut accumulator=0.0_f32;
    let mut mode=Order::Move;
    let mut zoom=1.0_f32;
    let mut center=vec2(WIDTH as f32*0.5,HEIGHT as f32*0.5);
    let mut pinch_prev: Option<(f32, Vec2)> = None;
    let mut map_gesture: Option<MapGesture> = None;
    let mut suppress_single_touch = false;
    let mut notice=String::from("Pinch to zoom, drag map to pan, tap to command.");
    let mut image=Image::gen_image_color(WIDTH as u16,HEIGHT as u16,BLACK);
    let texture=Texture2D::from_image(&image);
    // Smooth boundaries when magnified, rather than nearest-neighbor blocks.
    texture.set_filter(FilterMode::Linear);
    let mut last_texture_epoch=u64::MAX;
    let mut last_texture_winter=false;
    // Presentation-only positions: simulation remains authoritative/integer.
    let mut visual_positions: HashMap<u32, Vec2> = HashMap::new();
    let mut selected_supply=false;
    let mut supply_status_at=u64::MAX;
    let mut supply_status_id=0_u32;

    loop {
        let sw=screen_width();
        let sh=screen_height();
        let wide=sw>sh*1.24;
        let rail_w=if wide {(sw*0.32).clamp(285.,340.)} else {sw};
        let rail_x=if wide {sw-rail_w} else {0.};
        let controls_y=if wide {178.} else {(sh-211.).max(350.)};
        let board_region_w=if wide {rail_x-24.} else {sw-18.};
        let board_region_h=if wide {sh-108.} else {controls_y-98.};
        let map_scale=(board_region_w/WIDTH as f32).min(board_region_h/HEIGHT as f32);
        let board=Rect::new(
            if wide {12.+(board_region_w-WIDTH as f32*map_scale)*0.5}
            else {(sw-WIDTH as f32*map_scale)*0.5},
            82.+(board_region_h-HEIGHT as f32*map_scale)*0.4,
            WIDTH as f32*map_scale,HEIGHT as f32*map_scale,
        );
        let grid_side=7.;
        let button_w=(rail_w-24.-2.*grid_side)/3.;
        let button_h=49.;
        let buttons:Vec<Rect>=(0..9).map(|i|Rect::new(
            rail_x+12.+(i%3) as f32*(button_w+grid_side),
            controls_y+(i/3) as f32*57.,button_w,button_h)).collect();

        // Explicit gesture state: touching the map does nothing until the
        // finger lifts. Movement past a small threshold becomes camera pan.
        // This keeps a drag from ordering the selected army to move.
        let touch_list=touches();
        let active:Vec<_>=touch_list.iter()
            .filter(|t|!matches!(t.phase,TouchPhase::Ended|TouchPhase::Cancelled))
            .collect();
        let has_touch=!touch_list.is_empty();
        let pointer_start=if has_touch {
            touch_list.iter().find(|t|t.phase==TouchPhase::Started)
                .map(|t|t.position)
        } else if is_mouse_button_pressed(MouseButton::Left) {
            let (x,y)=mouse_position(); Some(vec2(x,y))
        } else {None};
        let pointer_current=if has_touch {
            active.first().map(|t|t.position)
        } else if is_mouse_button_down(MouseButton::Left) {
            let (x,y)=mouse_position(); Some(vec2(x,y))
        } else {None};
        let pointer_end=if has_touch {
            touch_list.iter().find(|t|t.phase==TouchPhase::Ended)
                .map(|t|t.position)
        } else if is_mouse_button_released(MouseButton::Left) {
            let (x,y)=mouse_position(); Some(vec2(x,y))
        } else {None};

        if active.len()>=2 {
            suppress_single_touch=true;
            map_gesture=None;
            let midpoint=(active[0].position+active[1].position)*0.5;
            let span=active[0].position.distance(active[1].position);
            if let Some((old_span,old_mid))=pinch_prev {
                pan_by_pixels(&mut center,zoom,board,midpoint-old_mid);
                if old_span>6.0 {
                    zoom_at(&mut center,&mut zoom,span/old_span,midpoint,board);
                }
            }
            pinch_prev=Some((span,midpoint));
        } else {pinch_prev=None;}
        if suppress_single_touch && active.is_empty() {
            suppress_single_touch=false;
        }
        let (mx,my)=mouse_position();
        let mouse_point=vec2(mx,my);
        let wheel=mouse_wheel().1;
        if wheel!=0.0 && board.contains(mouse_point) {
            zoom_at(&mut center,&mut zoom,(1.0+wheel*0.15).max(0.6),
                mouse_point,board);
        }

        if active.len()<2 && !suppress_single_touch {
            if let Some(p)=pointer_start {
                if let Some(index)=buttons.iter().position(|r|r.contains(p)) {
                match index {
                    0=>{running=!running;notice=if running {"Simulation running."}
                        else {"Paused. Issue orders, then resume."}.into();}
                    1=>{seed=seed.wrapping_add(1);world=World::new(seed);
                        selected=world.armies[0].id;mode=Order::Move;
                        zoom=1.;center=vec2(WIDTH as f32*0.5,HEIGHT as f32*0.5);
                        last_texture_epoch=u64::MAX;
                        map_gesture=None;
                        pinch_prev=None;
                        visual_positions.clear();
                        supply_status_at=u64::MAX;
                        running=true;
                        notice="New world generated. Your kingdom is BLUE.".into();}
                    2=>{
                        if let Some(id)=world.recruit(0) {
                            selected=id;
                            notice="New army recruited at capital. Tap a target.".into();
                        } else {
                            notice="Recruit needs 450 food, 180 population, max four armies.".into();
                        }
                    }
                    3=>{mode=Order::Move;
                        notice="MOVE selected. Tap a location on the map.".into();}
                    4=>{
                        if let Some(a)=player_army(&world,selected) {
                            let position=(a.x,a.y);
                            world.issue(0,selected,Order::Hold,position);
                            notice="Army holding position.".into();
                        } else {notice="Select or recruit an army first.".into();}
                        mode=Order::Hold;
                    }
                    5=>{mode=Order::Attack;
                        notice="ATTACK selected. Tap enemy land or capital.".into();}
                    6=>{
                        if let Some(a)=player_army(&world,selected) {
                            let target=(a.x,a.y);
                            world.issue(0,selected,Order::Retreat,target);
                            notice="Retreat ordered toward your capital.".into();
                        } else {notice="No living army selected.".into();}
                        mode=Order::Move;
                    }
                    7=>{zoom_at(&mut center, &mut zoom, 1.5, board.center(), board);}
                    8=>{zoom_at(&mut center, &mut zoom, 1.0/1.5, board.center(), board);}
                    _=>{}
                }
                } else if board.contains(p) {
                    map_gesture=Some(MapGesture{start:p,last:p,dragged:false});
                }
            }
            if let (Some(gesture),Some(current))=(map_gesture.as_mut(),pointer_current) {
                if (current-gesture.start).length()>=DRAG_THRESHOLD_PX {
                    gesture.dragged=true;
                }
                if gesture.dragged {
                    // Move in world coordinates: pan works while paused too.
                    let delta=current-gesture.last;
                    pan_by_pixels(&mut center,zoom,board,delta);
                }
                gesture.last=current;
            }
            if let Some(finish)=pointer_end {
                if let Some(gesture)=map_gesture.take() {
                    if !gesture.dragged &&
                        (finish-gesture.start).length()<DRAG_THRESHOLD_PX &&
                        board.contains(finish) {
                        let cell=map_cell(finish,board,view_rect(center,zoom));
                        // A tap near your army selects; tapping elsewhere
                        // issues one command only when the finger lifts.
                        let picked=world.armies.iter()
                            .filter(|a|a.owner==0 && a.troops>0)
                            .min_by_key(|a|(a.x-cell.0).abs()+(a.y-cell.1).abs())
                            .filter(|a|(a.x-cell.0).abs()+(a.y-cell.1).abs()<=2)
                            .map(|a|a.id);
                        if let Some(id)=picked {
                            selected=id;
                            notice=format!("Army #{id} selected. Tap its destination.");
                        } else if player_army(&world,selected).is_some() {
                            let command=if mode==Order::Hold {Order::Move} else {mode};
                            if world.issue(0,selected,command,cell) {
                                notice=format!("{} order sent to {},{}.",
                                    command.label(),cell.0,cell.1);
                                running=true;
                            } else {notice="Command rejected.".into();}
                        }
                    }
                }
            }
        }
        if is_key_pressed(KeyCode::Space){running=!running;}
        if is_key_pressed(KeyCode::R) {
            if let Some(id)=world.recruit(0) {selected=id;}
        }
        if is_key_pressed(KeyCode::Key1){mode=Order::Move;}
        if is_key_pressed(KeyCode::Key2){mode=Order::Attack;}
        if is_key_pressed(KeyCode::Key3){
            if let Some(a)=player_army(&world,selected){
                world.issue(0,selected,Order::Hold,(a.x,a.y));
            }
        }
        if is_key_pressed(KeyCode::Tab) {
            speed=match speed {4=>8,8=>16,_=>4};
            notice=format!("Speed: {speed} ticks per second.");
        }

        if running && world.winner.is_none() {
            accumulator+=get_frame_time().min(0.12);
            let delta=1./speed as f32;
            let mut n=0;
            while accumulator>=delta && n<5 {
                world.step();
                accumulator-=delta;
                n+=1;
            }
            if n==5 {accumulator=accumulator.min(delta);}
        } else {accumulator=0.;}
        current_army(&world,&mut selected);
        // Territory only changes during the three-tick simulation phase.
        // Avoid uploading a whole map texture on every simulation tick.
        let terrain_epoch = world.tick/3;
        if last_texture_epoch!=terrain_epoch || last_texture_winter!=world.winter() {
            refresh(&world,&mut image,&texture);
            last_texture_epoch=terrain_epoch;
            last_texture_winter=world.winter();
        }
        // No BFS through the world every 60-FPS presentation frame.
        if supply_status_at!=world.tick || supply_status_id!=selected {
            selected_supply=player_army(&world,selected)
                .is_some_and(|a|world.supply_connected(0,(a.x,a.y)));
            supply_status_at=world.tick;
            supply_status_id=selected;
        }
        let smoothing = 1.0 - (-get_frame_time().min(0.08)*11.0).exp();
        for a in &world.armies {
            let target=vec2(a.x as f32,a.y as f32);
            let current=visual_positions.entry(a.id).or_insert(target);
            // Only interpolate appearance; never modify authoritative position.
            if (*current-target).length()>12.0 {*current=target;}
            else {*current += (target-*current)*smoothing;}
        }
        visual_positions.retain(|id,_|world.armies.iter().any(|a|a.id==*id && a.troops>0));

        clear_background(Color::from_rgba(10,18,30,255));
        let heading=if wide {26.} else {(sw/16.).clamp(19.,27.)};
        draw_text("GENESIS  /  FRONTIER",12.,34.,heading,WHITE);
        draw_text(&format!("TICK {}  |  {}  |  {} TICKS/S",
            world.tick,if world.winter() {"WINTER"} else {"SUMMER"},speed),
            13.,57.,14.,Color::from_rgba(160,198,216,255));

        // Readable, minimal pixel-art geography; overlays are projections of
        // the same Rust world, not independent gameplay data.
        let view=view_rect(center,zoom);
        draw_texture_ex(&texture,board.x,board.y,WHITE,DrawTextureParams{
            source:Some(view),dest_size:Some(vec2(board.w,board.h)),
            ..Default::default()
        });
        draw_rectangle_lines(board.x,board.y,board.w,board.h,1.,
            Color::from_rgba(137,173,192,255));
        for i in 0..FACTIONS {
            if !world.kingdoms[i].alive {continue;}
            let cap=world.kingdoms[i].capital;
            let point=cell_to_screen(cap,board,view);
            if !board.contains(point){continue;}
            let color=kingdom_color(i as u8);
            draw_circle(point.x,point.y,8.5,color);
            draw_circle_lines(point.x,point.y,10.,1.6,WHITE);
            let text=world.kingdoms[i].name;
            let label_fs=if sw<370. {10.} else {12.};
            let label=measure_text(text,None,label_fs as u16,1.);
            draw_rectangle(point.x-label.width/2.-4.,point.y-29.,
                label.width+8.,17.,Color::from_rgba(14,26,38,200));
            draw_text(text,point.x-label.width*0.5,point.y-16.,label_fs,WHITE);
        }
        for a in world.armies.iter().filter(|a|a.troops>0 &&
            world.kingdoms[a.owner as usize].alive) {
            let visual=visual_positions.get(&a.id)
                .copied().unwrap_or(vec2(a.x as f32,a.y as f32));
            let p=screen_pos_from_world(visual,board,view);
            if !board.contains(p){continue;}
            let color=kingdom_color(a.owner);
            let radius=if a.owner==0 {8.} else {6.5};
            draw_circle(p.x,p.y,radius,Color::from_rgba(11,21,34,255));
            draw_circle_lines(p.x,p.y,radius+1.,2.,color);
            draw_circle(p.x,p.y,3.,color);
            if a.owner==0 && a.id==selected {
                draw_circle_lines(p.x,p.y,radius+5.,2.,Color::from_rgba(133,242,255,255));
                let dest=cell_to_screen(a.target,board,view);
                if board.contains(dest) {
                    draw_line(p.x,p.y,dest.x,dest.y,1.8,
                        Color::from_rgba(121,226,243,220));
                    draw_circle_lines(dest.x,dest.y,8.,1.3,
                        Color::from_rgba(121,226,243,230));
                }
            }
            let count=format!("{}",a.troops);
            let size=11.;
            let measured=measure_text(&count,None,size as u16,1.);
            let tx=p.x-measured.width*0.5;
            draw_rectangle(tx-3.,p.y-25.,measured.width+6.,14.,
                Color::from_rgba(16,26,39,226));
            draw_text(&count,tx,p.y-14.,size,WHITE);
        }

        let counts=world.counts();
        let player=&world.kingdoms[0];
        let stats=format!("BLUE  POP {}  FOOD {}  LAND {}  ARMIES {}",
            player.population,player.food.max(0),counts[0],
            world.armies.iter().filter(|a|a.owner==0 && a.troops>0).count());
        if wide {
            draw_text("YOUR KINGDOM",rail_x+12.,106.,18.,WHITE);
            draw_text(&format!("POP {}   FOOD {}",player.population,player.food),
                rail_x+12.,131.,15.,Color::from_rgba(176,221,231,255));
            draw_text(&format!("LAND {}   ARMY {}",counts[0],
                player_army(&world,selected).map_or(0,|a|a.troops)),
                rail_x+12.,153.,15.,LIGHTGRAY);
        } else {
            let y=(board.y+board.h+18.).min(controls_y-49.);
            draw_text(&stats,10.,y,(sw/33.).clamp(10.,14.),WHITE);
            let select_text=if let Some(a)=player_army(&world,selected){
                format!("Selected army #{}: {} troops | {}",
                    a.id,a.troops,if selected_supply{"SUPPLIED"}
                    else {"CUT OFF"})
            } else {"No army remaining. Recruit at capital.".into()};
            draw_text(&select_text,10.,y+17.,(sw/35.).clamp(10.,13.),
                Color::from_rgba(170,221,230,255));
        }

        let labels=[if running {"PAUSE"} else {"PLAY"},"NEW WORLD","RECRUIT",
            "MOVE","HOLD","ATTACK","RETREAT","ZOOM +","ZOOM -"];
        for (i,r) in buttons.iter().enumerate() {
            let active=match i {0=>running,3=>mode==Order::Move,
                4=>mode==Order::Hold,5=>mode==Order::Attack,_=>false};
            draw_button(*r,labels[i],active);
        }

        let footer_y=if wide {controls_y+3.*57.+31.} else {sh-20.};
        let instruction=if !world.kingdoms[0].alive {
            "DEFEAT — Your capital was captured."
        } else if world.winner==Some(0) {
            "VICTORY — Your kingdom dominates the world!"
        } else if world.winner.is_some() {
            "MATCH OVER — Another kingdom won."
        } else {"TAP: COMMAND   DRAG: PAN   PINCH: ZOOM"};
        let instruction_y=if wide {controls_y+3.*57.+11.}
            else {controls_y-14.};
        draw_text(instruction,if wide {rail_x+10.} else {12.},
            instruction_y,(rail_w/31.).clamp(10.,13.),
            Color::from_rgba(145,232,236,255));
        let limit=if wide {44} else {52};
        let truncated:String=notice.chars().take(limit).collect();
        draw_text(&truncated,if wide {rail_x+10.} else {12.},
            footer_y,(rail_w/37.).clamp(10.,13.),LIGHTGRAY);

        if world.winner.is_some() && !wide {
            draw_rectangle(board.x+13.,board.y+board.h*0.43,
                board.w-26.,54.,Color::from_rgba(14,28,42,230));
            let winner=world.winner.unwrap();
            let text=if winner==0 {"YOU WIN"} else {"YOU LOSE"};
            let tw=measure_text(text,None,28,1.);
            draw_text(text,board.x+(board.w-tw.width)/2.,
                board.y+board.h*0.43+35.,28.,
                if winner==0 {Color::from_rgba(139,239,194,255)}
                else {Color::from_rgba(248,149,130,255)});
        }
        next_frame().await;
    }
}
