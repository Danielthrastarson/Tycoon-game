//! Genesis Frontier — readable competitive strategy, live Rust engine.
//! Play against three computer-controlled kingdoms. Real online multiplayer
//! remains a later networking milestone.
use genesis::frontier::{World, Terrain, Order, WIDTH, HEIGHT, FACTIONS, NEUTRAL, NORTH_CROSSING};
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

// Simulation cells are low-resolution *state*, not display pixels.
// Each logical cell gets four presentation pixels with gentle cartographic
// detail and a thin coast/territorial boundary. No physics is smoothed.
const MAP_RASTER: usize = 4;

fn refresh(world:&World,image:&mut Image,texture:&Texture2D) {
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let tile=world.tiles[y*WIDTH+x];
            let base=terrain_color(tile.terrain,world.winter());
            let mut ground=if tile.owner==NEUTRAL || tile.terrain==Terrain::Sea {
                base
            } else {
                blend(base,kingdom_color(tile.owner),
                    if tile.terrain==Terrain::Mountain {0.25} else {0.60})
            };
            // Quiet terrain variation reads as texture rather than a grid.
            let variation=((x*17+y*31+world.seed as usize%19)%13) as f32;
            ground=blend(ground,Color::from_rgba(209,222,193,255),
                variation/260.0);
            let east=if x+1<WIDTH {world.tiles[y*WIDTH+x+1]} else {tile};
            let south=if y+1<HEIGHT {world.tiles[(y+1)*WIDTH+x]} else {tile};
            let owner_edge_e=tile.owner!=east.owner && tile.terrain.traversable()
                && east.terrain.traversable();
            let owner_edge_s=tile.owner!=south.owner && tile.terrain.traversable()
                && south.terrain.traversable();
            let coast_e=tile.terrain.traversable()!=east.terrain.traversable();
            let coast_s=tile.terrain.traversable()!=south.terrain.traversable();
            for sy in 0..MAP_RASTER {
                for sx in 0..MAP_RASTER {
                    let mut c=ground;
                    let edge= (sx==MAP_RASTER-1 && (owner_edge_e || coast_e))
                        || (sy==MAP_RASTER-1 && (owner_edge_s || coast_s));
                    if edge {
                        // Thin warm outlines are clearer than block edges.
                        let ink=if tile.owner==NEUTRAL {
                            Color::from_rgba(142,171,161,255)
                        } else {blend(kingdom_color(tile.owner),WHITE,0.38)};
                        c=blend(c,ink,if coast_e||coast_s {0.50} else {0.65});
                    } else {
                        // Tiny marks hint at geography without drawing icons
                        // for thousands of tiles every render frame.
                        let detail=match tile.terrain {
                            Terrain::Mountain=>sx==2 && sy==1,
                            Terrain::Forest=>sx==1 && sy==2,
                            Terrain::Farm=>sy==2 && (sx==1||sx==3),
                            _=>false
                        };
                        if detail {
                            c=blend(c,if tile.terrain==Terrain::Mountain {
                                Color::from_rgba(222,226,224,255)
                            } else if tile.terrain==Terrain::Forest {
                                Color::from_rgba(18,60,50,255)
                            } else {
                                Color::from_rgba(213,220,147,255)
                            },0.42);
                        }
                    }
                    image.set_pixel((x*MAP_RASTER+sx) as u32,
                        (y*MAP_RASTER+sy) as u32,c);
                }
            }
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


/// Temporary presentation-only information about an actual world change.
struct VisualEvent {
    position: Vec2,
    label: String,
    color: Color,
    at: f64,
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
    let mut notice=String::from("Your forward army is selected. Secure the northern crossing.");
    let mut visual_events:Vec<VisualEvent>=Vec::new();
    let mut cached_route:Vec<(i16,i16)>=Vec::new();
    let mut cached_route_key:Option<(u32,(i16,i16),(i16,i16))>=None;
    let mut image=Image::gen_image_color((WIDTH*MAP_RASTER) as u16,
        (HEIGHT*MAP_RASTER) as u16,BLACK);
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
        // Some mobile browsers deliver the touch and synthetic mouse
        // transitions in different frames. Preserve the old mouse fallback
        // instead of suppressing it merely because a touch entry exists.
        let pointer_start=touch_list.iter()
            .find(|t|t.phase==TouchPhase::Started)
            .map(|t|t.position)
            .or_else(|| if is_mouse_button_pressed(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            } else {None});
        let pointer_current=active.first()
            .map(|t|t.position)
            .or_else(|| if is_mouse_button_down(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            } else {None});
        let pointer_end=touch_list.iter()
            .find(|t|t.phase==TouchPhase::Ended)
            .map(|t|t.position)
            .or_else(|| if is_mouse_button_released(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            } else {None});

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
                        visual_events.clear();
                        cached_route_key=None;
                        cached_route.clear();
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
                // These effects are derived from *actual* battles/captures,
                // never scripted progress or unearned visual territory.
                if world.last_report.captured[0] > 0 {
                    if let Some(a)=world.armies.iter().find(|a|a.owner==0 &&
                        a.troops>0 && a.order!=Order::Hold) {
                        visual_events.push(VisualEvent {
                            position:vec2(a.x as f32,a.y as f32),
                            label:format!("+{} LAND",world.last_report.captured[0]),
                            color:Color::from_rgba(145,248,210,255),
                            at:get_time(),
                        });
                    }
                }
                if world.last_report.clashes > 0 {
                    if let Some(a)=world.armies.iter().find(|a|a.troops>0 &&
                        world.armies.iter().any(|b|b.owner!=a.owner && b.troops>0 &&
                            (b.x-a.x).abs()+(b.y-a.y).abs()<=1)) {
                        let losses: u32=world.last_report.casualties.iter().sum();
                        visual_events.push(VisualEvent {
                            position:vec2(a.x as f32,a.y as f32),
                            label:format!("BATTLE -{losses}"),
                            color:Color::from_rgba(255,177,140,255),
                            at:get_time(),
                        });
                    }
                }
                if let Some((attacker,defender))=world.last_report.capital_taken {
                    notice=format!("{} CAPTURED {}'S CAPITAL!",
                        world.kingdoms[attacker as usize].name,
                        world.kingdoms[defender as usize].name);
                }
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
        visual_events.retain(|effect|get_time()-effect.at<1.35);
        // Pathfinding is cached on movement/order changes; never recompute
        // an entire shortest path inside every display frame.
        if let Some(a)=player_army(&world,selected) {
            let key=(selected,(a.x,a.y),a.target);
            if cached_route_key!=Some(key) {
                cached_route=world.route((a.x,a.y),a.target);
                cached_route_key=Some(key);
            }
        } else {cached_route.clear();cached_route_key=None;}

        clear_background(Color::from_rgba(10,18,30,255));
        let heading=if wide {26.} else {(sw/16.).clamp(19.,27.)};
        draw_text("GENESIS  /  FRONTIER",12.,34.,heading,WHITE);
        draw_text(&format!("TICK {}  |  {}  |  {} TICKS/S",
            world.tick,if world.winter() {"WINTER"} else {"SUMMER"},speed),
            13.,55.,13.,Color::from_rgba(160,198,216,255));
        let secured=world.owner_at(NORTH_CROSSING.0,NORTH_CROSSING.1)==0;
        let mission=if secured {"CROSSING SECURED - PUSH INTO ENEMY LAND"}
            else {"OBJECTIVE: CAPTURE NORTHERN CROSSING"};
        draw_text(mission,13.,73.,(sw/33.5).clamp(10.,13.),
            if secured {Color::from_rgba(140,239,197,255)}
            else {Color::from_rgba(252,205,135,255)});

        // Readable, minimal pixel-art geography; overlays are projections of
        // the same Rust world, not independent gameplay data.
        let view=view_rect(center,zoom);
        draw_texture_ex(&texture,board.x,board.y,WHITE,DrawTextureParams{
            source:Some(Rect::new(view.x*MAP_RASTER as f32,
                view.y*MAP_RASTER as f32,
                view.w*MAP_RASTER as f32,view.h*MAP_RASTER as f32)),
            dest_size:Some(vec2(board.w,board.h)),
            ..Default::default()
        });
        draw_rectangle_lines(board.x,board.y,board.w,board.h,1.,
            Color::from_rgba(137,173,192,255));

        // The crossing is a visible, immediately meaningful first goal.
        let crossing=cell_to_screen(NORTH_CROSSING,board,view);
        if board.contains(crossing) {
            let pulse=((get_time() as f32*3.0).sin()+1.0)*0.5;
            let edge=if secured {Color::from_rgba(136,251,200,255)}
                else {Color::from_rgba(254,211,130,255)};
            draw_circle_lines(crossing.x,crossing.y,8.0+pulse*3.5,1.7,edge);
            draw_circle(crossing.x,crossing.y,3.0,edge);
            if zoom>=1.4 {
                let t=if secured {"CROSSING: OURS"} else {"TAKE CROSSING"};
                let size=11.0;
                let m=measure_text(t,None,11,1.0);
                let x=(crossing.x-m.width*0.5).clamp(board.x+3.0,
                    (board.x+board.w-m.width-3.0).max(board.x+3.0));
                draw_rectangle(x-3.0,crossing.y-28.0,m.width+6.0,16.0,
                    Color::from_rgba(19,34,45,220));
                draw_text(t,x,crossing.y-16.0,size,edge);
            }
        }

        // Route follows the authoritative Rust shortest path, not a direct
        // line through mountains or impassable water.
        if let Some(a)=player_army(&world,selected) {
            let current=visual_positions.get(&a.id)
                .copied().unwrap_or(vec2(a.x as f32,a.y as f32));
            let mut prev=screen_pos_from_world(current,board,view);
            for (i,step) in cached_route.iter().enumerate() {
                let next=cell_to_screen(*step,board,view);
                if board.contains(prev) && board.contains(next) {
                    draw_line(prev.x,prev.y,next.x,next.y,2.0,
                        Color::from_rgba(12,30,38,220));
                    if (i as u64+world.tick/2)%3 != 0 {
                        draw_line(prev.x,prev.y,next.x,next.y,1.7,
                            Color::from_rgba(111,227,246,245));
                    }
                }
                prev=next;
            }
        }
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
            // A small moving troop formation is much more legible than one
            // lonely dot. Combat and motion still come from the Rust sim.
            let heading=vec2(a.target.0 as f32-visual.x,
                a.target.1 as f32-visual.y);
            let direction=if heading.length()>0.2 {
                heading.normalize()
            } else {vec2(1.0,0.0)};
            let side=vec2(-direction.y,direction.x);
            let spacing=(board.w/view.w*0.32).clamp(3.0,6.0);
            draw_circle(p.x,p.y,radius+2.5,Color::from_rgba(10,20,32,220));
            for (ahead,across) in [(-1.0,-0.7),(-1.0,0.7),
                (0.0,-0.95),(0.0,0.95),(0.85,0.0)] {
                let wobble=(get_time() as f32*7.0+ahead*3.0+across).sin()*0.4;
                let offset=direction*(ahead*spacing+wobble)+side*across*spacing;
                draw_circle(p.x+offset.x,p.y+offset.y,2.0,color);
            }
            draw_circle_lines(p.x,p.y,radius+1.0,1.5,
                if a.owner==0 && a.id==selected {WHITE}else{color});
            let bar_width=24.0;
            let strength=(a.troops as f32/300.0).clamp(0.0,1.0);
            draw_rectangle(p.x-bar_width*0.5,p.y+radius+6.0,
                bar_width,3.0,Color::from_rgba(15,26,38,220));
            draw_rectangle(p.x-bar_width*0.5,p.y+radius+6.0,
                bar_width*strength,3.0,
                if strength>0.4 {Color::from_rgba(147,238,196,255)}
                else {Color::from_rgba(252,169,116,255)});
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

        // Quick feedback for real captures and clashes: rises and fades.
        for effect in &visual_events {
            let elapsed=(get_time()-effect.at) as f32;
            let pos=screen_pos_from_world(effect.position,board,view);
            if !board.contains(pos) {continue;}
            let mut tint=effect.color;
            tint.a=(1.0-elapsed/1.35).clamp(0.0,1.0);
            let font=13.0;
            let measurement=measure_text(&effect.label,None,13,1.0);
            let px=pos.x-measurement.width*0.5;
            let py=pos.y-22.0-elapsed*16.0;
            draw_rectangle(px-4.0,py-14.0,measurement.width+8.0,18.0,
                Color::new(0.04,0.09,0.14,0.7*tint.a));
            draw_text(&effect.label,px,py,font,tint);
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
