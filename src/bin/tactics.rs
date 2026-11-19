//! Genesis Frontier V0.4.4 — first tactile battle experience.
//! One human commands the two blue squads. Opposing armies use the exact
//! authoritative Rust combat simulator and pre-existing counter AI.
//! Rendering and effects only observe outcomes; never create victories.
use genesis::frontier::{Order, Terrain};
use genesis::frontier_combat::{
    Battle, CombatRules, Layout, OpponentPolicy, Policy, Scenario,
    VictoryReason, FIELD_WIDTH, FIELD_HEIGHT, MAX_ROUNDS, MOUNTAIN_DEFEND_ROUNDS,
};
use macroquad::prelude::*;

fn config()->Conf {
    Conf {
        window_title:"Genesis — Tactical Siege".into(),
        window_width:440,window_height:840,
        high_dpi:false,
        ..Default::default()
    }
}

fn rgba(r:u8,g:u8,b:u8)->Color {Color::from_rgba(r,g,b,255)}
fn tint(c:Color,k:f32)->Color {
    Color::new((c.r*k).min(1.0),(c.g*k).min(1.0),(c.b*k).min(1.0),c.a)
}
const BG:Color=Color{r:0.045,g:0.073,b:0.108,a:1.0};
const CYAN:Color=Color{r:0.38,g:0.84,b:1.0,a:1.0};
const ORANGE:Color=Color{r:1.0,g:0.62,b:0.37,a:1.0};
const GOLD:Color=Color{r:0.96,g:0.83,b:0.48,a:1.0};
const GREEN:Color=Color{r:0.43,g:0.89,b:0.67,a:1.0};
const MUTED:Color=Color{r:0.55,g:0.66,b:0.74,a:1.0};
const INK:Color=Color{r:0.08,g:0.13,b:0.18,a:1.0};

fn text(label:&str,x:f32,y:f32,size:f32,color:Color) {
    draw_text(label,x,y,size,color);
}
fn center_text(label:&str,center:Vec2,size:f32,color:Color) {
    let dim=measure_text(label,None,size as u16,1.0);
    text(label,center.x-dim.width/2.0,center.y+dim.height/3.0,size,color);
}
fn button(rect:Rect,label:&str,selected:bool)->() {
    draw_rectangle(rect.x,rect.y,rect.w,rect.h,
        if selected {rgba(33,111,135)} else {rgba(31,47,64)});
    draw_rectangle_lines(rect.x+0.5,rect.y+0.5,rect.w-1.,rect.h-1.,1.4,
        if selected {CYAN}else{rgba(68,90,110)});
    center_text(label,rect.center(),(rect.w/(label.len() as f32*0.64+1.))
        .clamp(11.,16.),if selected {WHITE}else{rgba(203,218,227)});
}

#[derive(Clone,Copy)]
struct Gesture {origin:Vec2,last:Vec2,dragging:bool}
struct Floater {at:Vec2,message:String,born:f64,color:Color}

fn world_view(center:Vec2,zoom:f32,area:Rect)->Rect {
    let mut height=(FIELD_HEIGHT as f32/zoom).clamp(3.8,FIELD_HEIGHT as f32);
    let mut width=height*area.w/area.h;
    if width>FIELD_WIDTH as f32 {
        width=FIELD_WIDTH as f32;
        height=width*area.h/area.w;
    }
    Rect::new(
        (center.x-width*0.5).clamp(0.,(FIELD_WIDTH as f32-width).max(0.)),
        (center.y-height*0.5).clamp(0.,(FIELD_HEIGHT as f32-height).max(0.)),
        width,height
    )
}
fn clamp_camera(center:&mut Vec2,zoom:f32,area:Rect) {
    let view=world_view(*center,zoom,area);
    *center=vec2(view.x+view.w*0.5,view.y+view.h*0.5);
}
fn pan(center:&mut Vec2,zoom:f32,area:Rect,delta:Vec2) {
    let view=world_view(*center,zoom,area);
    center.x-=delta.x*view.w/area.w;
    center.y-=delta.y*view.h/area.h;
    clamp_camera(center,zoom,area);
}
fn zoom_on(center:&mut Vec2,zoom:&mut f32,factor:f32,at:Vec2,area:Rect) {
    let before=world_view(*center,*zoom,area);
    let x=((at.x-area.x)/area.w).clamp(0.,1.);
    let y=((at.y-area.y)/area.h).clamp(0.,1.);
    let pt=vec2(before.x+before.w*x,before.y+before.h*y);
    *zoom=(*zoom*factor).clamp(1.,2.6);
    let after=world_view(*center,*zoom,area);
    *center+=pt-vec2(after.x+after.w*x,after.y+after.h*y);
    clamp_camera(center,*zoom,area);
}
fn screen(p:Vec2,area:Rect,view:Rect)->Vec2 {
    vec2(area.x+(p.x+0.5-view.x)*area.w/view.w,
         area.y+(p.y+0.5-view.y)*area.h/view.h)
}
fn cell_from(p:Vec2,area:Rect,view:Rect)->(i16,i16) {
    let x=(view.x+(p.x-area.x)*view.w/area.w).floor()
        .clamp(0.,(FIELD_WIDTH-1) as f32) as i16;
    let y=(view.y+(p.y-area.y)*view.h/area.h).floor()
        .clamp(0.,(FIELD_HEIGHT-1) as f32) as i16;
    (x,y)
}
fn sq(p:(i16,i16),area:Rect,view:Rect)->Vec2 {
    screen(vec2(p.0 as f32,p.1 as f32),area,view)
}
fn terrain_color(t:Terrain)->Color {
    match t {
        Terrain::Sea=>rgba(27,83,118),
        Terrain::Plain=>rgba(62,104,86),
        Terrain::Farm=>rgba(91,135,76),
        Terrain::Forest=>rgba(34,91,76),
        Terrain::Mountain=>rgba(108,125,127),
    }
}
fn interpolated(a:(i16,i16),b:(i16,i16),alpha:f32)->Vec2 {
    vec2(a.0 as f32,a.1 as f32).lerp(
        vec2(b.0 as f32,b.1 as f32),alpha)
}
fn map_render(b:&Battle,area:Rect,view:Rect,positions:[Vec2;4],
    chosen:usize,hover:Option<(i16,i16)>,mode:Order,
    effects:&[Floater]) {
    draw_rectangle(area.x,area.y,area.w,area.h,rgba(13,29,39));
    let cell=area.w/view.w;
    let start_x=view.x.floor() as i16;
    let end_x=(view.x+view.w).ceil().min(FIELD_WIDTH as f32) as i16;
    let start_y=view.y.floor() as i16;
    let end_y=(view.y+view.h).ceil().min(FIELD_HEIGHT as f32) as i16;
    for y in start_y.max(0)..end_y {
        for x in start_x.max(0)..end_x {
            let t=b.terrain_at((x,y));
            let center=sq((x,y),area,view);
            let left=center.x-cell*0.5;
            let top=center.y-cell*0.5;
            let rough=((x as i32*19+y as i32*11+b.seed as i32)%9) as f32;
            // Clip partially visible terrain cells to the map viewport.
            let rx=left.max(area.x);
            let ry=top.max(area.y);
            let rw=(left+cell+0.7).min(area.x+area.w)-rx;
            let rh=(top+cell+0.7).min(area.y+area.h)-ry;
            if rw>0. && rh>0. {
                draw_rectangle(rx,ry,rw,rh,
                    tint(terrain_color(t),0.93+rough*0.012));
            }
            let fully_visible=left>=area.x && top>=area.y &&
                left+cell<=area.x+area.w && top+cell<=area.y+area.h;
            if cell>=16. && fully_visible {
                match t {
                    Terrain::Mountain=>{
                        let d=cell*0.30;
                        draw_triangle(vec2(center.x,center.y-d),
                            vec2(center.x-d*0.91,center.y+d*0.60),
                            vec2(center.x+d*0.91,center.y+d*0.60),
                            rgba(163,179,176));
                        draw_line(center.x,center.y-d,
                            center.x+d*0.20,center.y+d*0.23,1.2,rgba(225,229,211));
                    }
                    Terrain::Forest=>{
                        for off in [-0.24_f32,0.17] {
                            draw_circle(center.x+cell*off,center.y+cell*0.02,
                                (cell*0.135).max(1.9),rgba(19,68,57));
                            draw_circle(center.x+cell*off,center.y-cell*0.07,
                                (cell*0.10).max(1.7),rgba(39,124,86));
                        }
                    }
                    Terrain::Sea=>{
                        draw_line(left+cell*0.22,center.y-cell*0.08,
                            left+cell*0.64,center.y-cell*0.08,1.,rgba(62,138,161));
                    }
                    _=>{}
                }
            }
            if fully_visible {
                draw_rectangle_lines(left,top,cell+0.5,cell+0.5,0.38,
                    Color::new(0.80,0.93,0.83,0.085));
            }
        }
    }
    // This soft boundary only clips overlay objects; terrain tiles are
    // already restricted to the board's visible source range.
    let objective=sq(b.objective,area,view);
    if area.contains(objective) {
        let pulse=(get_time() as f32*2.4).sin()*0.5+0.5;
        let edge=if b.units[0].pos==b.objective {GREEN}else{GOLD};
        draw_circle_lines(objective.x,objective.y,cell*0.32+pulse*3.4,2.4,edge);
        draw_circle(objective.x,objective.y,3.0,edge);
        let label="HOLD PASS";
        if objective.y>area.y+36. {
            let width=measure_text(label,None,12,1.).width;
            draw_rectangle(objective.x-width*0.5-6.,objective.y-cell*0.5-22.,
                width+12.,17.,Color::new(0.07,0.11,0.14,0.92));
            text(label,objective.x-width*0.5,objective.y-cell*0.5-9.,12.,GOLD);
        }
    }
    // Physical depot markers. Supply connectivity is calculated by the
    // existing corridor model, not by painting an imaginary road on water.
    for side in 0..2 {
        let p=sq(b.depots[side],area,view);
        if !area.contains(p) {continue;}
        let color=if side==0 {CYAN}else{ORANGE};
        draw_rectangle(p.x-6.,p.y-6.,12.,12.,rgba(16,29,38));
        draw_rectangle_lines(p.x-6.,p.y-6.,12.,12.,2.,color);
        text(if side==0 {"SUPPLY"} else {"ENEMY"},
            (p.x-19.0).clamp(area.x+3.0,area.x+area.w-49.0),
            p.y+20.0,9.0,color);
    }
    // Each squad has a legible supply indicator; colored corridor-status
    // links are not claimed to be pathfinding-geographical supply routes.
    for i in 0..4 {
        let u=b.units[i];
        if u.troops==0 {continue;}
        let connected=b.connected(i);
        let source=sq(b.depots[u.side as usize],area,view);
        let destination=screen(positions[i],area,view);
        if area.contains(source)&&area.contains(destination) {
            let color=if connected {GREEN}else{rgba(236,108,94)};
            let displacement=destination-source;
            let segments=(displacement.length()/13.).ceil().max(1.) as i32;
            for j in 0..segments {
                if j%2==0 {
                    let p=source+displacement*j as f32/segments as f32;
                    let q=source+displacement*(j+1) as f32/segments as f32;
                    draw_line(p.x,p.y,q.x,q.y,1.0,Color::new(color.r,color.g,color.b,0.24));
                }
            }
        }
    }
    // The actual route is obtained from the authoritative BFS router.
    if b.units[chosen].troops>0 {
        let mut last=screen(positions[chosen],area,view);
        for (n,location) in b.route_preview(chosen).iter().enumerate() {
            let next=sq(*location,area,view);
            if area.contains(last)&&area.contains(next) {
                draw_line(last.x,last.y,next.x,next.y,2.2,
                    Color::new(0.04,0.15,0.19,0.90));
                if (n+((get_time()*5.) as usize))%3!=0 {
                    draw_line(last.x,last.y,next.x,next.y,1.5,CYAN);
                }
            }
            last=next;
        }
    }
    if let Some(p)=hover {
        let at=sq(p,area,view);
        if area.contains(at) {
            let color=if b.terrain_at(p)==Terrain::Sea {rgba(242,100,100)}
                else if mode==Order::Attack {ORANGE}else{CYAN};
            draw_rectangle_lines(at.x-cell*0.5,at.y-cell*0.5,cell,cell,1.6,color);
        }
    }

    for i in 0..4 {
        let u=b.units[i];
        if u.troops==0 {continue;}
        let pos=screen(positions[i],area,view);
        if !area.contains(pos) {continue;}
        let color=if u.side==0 {CYAN}else{ORANGE};
        let r=(cell*0.245).clamp(7.,18.);
        draw_circle(pos.x,pos.y,r+4.,Color::new(0.02,0.06,0.09,0.83));
        let gap=r*0.65;
        for (ox,oy) in [(-0.75,-0.2),(0.75,-0.2),(-0.42,0.65),(0.42,0.65),(0.,-0.75)] {
            draw_circle(pos.x+ox*gap,pos.y+oy*gap,
                (r*0.25).max(1.8),color);
        }
        draw_circle_lines(pos.x,pos.y,r+2.,
            if i==chosen {2.7}else{1.3},
            if i==chosen {WHITE}else{color});
        if i==chosen {
            draw_circle_lines(pos.x,pos.y,r+6.,1.2,CYAN);
        }
        let width=(r*2.4).max(22.);
        let fraction=(u.troops as f32/(if u.side==0 {300.}else{520.}))
            .clamp(0.,1.);
        draw_rectangle(pos.x-width*0.5,pos.y+r+7.,width,4.,rgba(22,38,46));
        draw_rectangle(pos.x-width*0.5,pos.y+r+7.,width*fraction,4.,
            if fraction>0.30 {GREEN}else{ORANGE});
        let label=format!("{}",u.troops);
        center_text(&label,vec2(pos.x,pos.y-r-11.),13.,WHITE);
        if u.routing {
            text("ROUT",pos.x-r-9.,pos.y+r+23.,10.,ORANGE);
        }
    }
    let now=get_time();
    for fx in effects {
        let elapsed=(now-fx.born) as f32;
        if elapsed>1.5 {continue;}
        let pos=screen(fx.at,area,view);
        if !area.contains(pos) {continue;}
        let opacity=(1.-elapsed/1.5).clamp(0.,1.);
        let color=Color::new(fx.color.r,fx.color.g,fx.color.b,opacity);
        let dim=measure_text(&fx.message,None,15,1.);
        draw_rectangle(pos.x-dim.width*0.5-4.,
            pos.y-33.-elapsed*13.-15.,dim.width+8.,19.,
            Color::new(0.05,0.10,0.13,0.70*opacity));
        center_text(&fx.message,
            vec2(pos.x,pos.y-33.-elapsed*13.),15.,color);
    }
    draw_rectangle_lines(area.x,area.y,area.w,area.h,1.5,rgba(93,123,141));
}
/// The full map remains visible even on tall portrait screens. Tapping
/// this miniature recenters the authoritative world camera.
fn minimap_rect(area:Rect)->Rect {
    let width=(area.w*0.32).clamp(110.0,155.0).min(area.w-12.0);
    let height=width*FIELD_HEIGHT as f32/FIELD_WIDTH as f32;
    Rect::new(area.x+area.w-width-9.0,area.y+9.0,width,height)
}
fn draw_minimap(b:&Battle,area:Rect,view:Rect) {
    let m=minimap_rect(area);
    draw_rectangle(m.x-5.0,m.y-5.0,m.w+10.0,m.h+24.0,
        Color::new(0.055,0.09,0.13,0.95));

    let cw=m.w/FIELD_WIDTH as f32;
    let ch=m.h/FIELD_HEIGHT as f32;
    for y in 0..FIELD_HEIGHT {
        for x in 0..FIELD_WIDTH {
            draw_rectangle(m.x+x as f32*cw,m.y+y as f32*ch,
                cw+0.15,ch+0.15,terrain_color(b.terrain_at((x,y))));
        }
    }
    for u in &b.units {
        if u.troops==0 {continue;}
        let cx=m.x+(u.pos.0 as f32+0.5)*cw;
        let cy=m.y+(u.pos.1 as f32+0.5)*ch;
        draw_circle(cx,cy,2.9,if u.side==0 {CYAN}else{ORANGE});
    }
    draw_rectangle_lines(
        m.x+view.x*cw,m.y+view.y*ch,
        view.w*cw,view.h*ch,1.4,WHITE);
    draw_rectangle_lines(m.x,m.y,m.w,m.h,1.0,rgba(124,151,164));
    text("TAP TO PAN",m.x+3.0,m.y+m.h+14.0,10.0,rgba(185,217,230));
}

fn reset(seed:u64,layout:Layout)->Battle {
    Battle::new(seed,Scenario::MountainPass,Policy::Adaptive,false,
        CombatRules::FULL)
        .with_layout(layout)
        .with_opponent(OpponentPolicy::Counter)
}
fn primary_state(b:&Battle,chosen:usize)->String {
    let u=b.units[chosen];
    let supplies=if b.connected(chosen) {"CONNECTED"} else {"CUT OFF"};
    format!("{} TROOPS  |  MORALE {}  |  FATIGUE {}  |  FOOD {}/6  |  {}",
        u.troops,u.state.morale,u.state.fatigue,u.state.provisions,supplies)
}
fn cause_label(cause:Option<VictoryReason>)->&'static str {
    match cause {
        Some(VictoryReason::PassHeld)=>"The mountain pass held for 24 rounds",
        Some(VictoryReason::Breakthrough)=>"Enemy occupied your rear breakthrough tile",
        Some(VictoryReason::SupplyInterdicted)=>"Supply corridor was held under interdiction",
        Some(VictoryReason::DepotCapture)=>"An undefended depot was captured",
        Some(VictoryReason::Annihilation)=>"An army collapsed below 40 soldiers",
        Some(VictoryReason::CenterSecured)=>"The central objective was secured",
        None=>"No army secured an objective in 80 rounds",
    }
}
fn board_layout(sw:f32,sh:f32)->(Rect,[Rect;2],[Rect;4],[Rect;5],bool) {
    let wide=sw>=sh*1.22;
    let control_width=if wide {(sw*0.31).clamp(280.,345.)}else{sw-20.};
    let origin_x=if wide {sw-control_width-10.}else{10.};
    let bottom_start=if wide {142.}else{(sh-207.).max(426.)};
    let area=if wide {
        Rect::new(12.,111.,(origin_x-24.).max(170.),(sh-129.).max(140.))
    }else{
        // Reserve 42 px above the action rail for objective and squad stats.
        // Otherwise those labels overlap the selection buttons on phones.
        Rect::new(10.,108.,(sw-20.).max(160.),
            (bottom_start-160.).max(155.))
    };
    let gap=7.;
    let squad_width=(control_width-gap)/2.;
    let squad=[
        Rect::new(origin_x,bottom_start,squad_width,42.),
        Rect::new(origin_x+squad_width+gap,bottom_start,squad_width,42.)
    ];
    let cmd_width=(control_width-gap*3.)/4.;
    let cmd=std::array::from_fn(|i|Rect::new(
        origin_x+i as f32*(cmd_width+gap),bottom_start+49.,cmd_width,47.));
    let utility_width=(control_width-gap*4.)/5.;
    let utility=std::array::from_fn(|i|Rect::new(
        origin_x+i as f32*(utility_width+gap),bottom_start+103.,
        utility_width,38.));
    (area,squad,cmd,utility,wide)
}

#[macroquad::main(config)]
async fn main() {
    let mut seed=1001_u64;
    let layouts=[Layout::Ridge,Layout::RiverFord,Layout::SplitValley];
    let mut layout_idx=0usize;
    let mut battle=reset(seed,layouts[layout_idx]);
    let mut previous_positions:[(i16,i16);4]=std::array::from_fn(|i|
        battle.units[i].pos);
    let mut chosen=0usize;
    let mut mode=Order::Hold;
    let mut running=false;
    let mut fast=false;
    let mut accumulator=0_f32;
    let mut zoom=1.0_f32;
    let mut camera=vec2(7.,4.);
    let mut gesture:Option<Gesture>=None;
    let mut pinch_prev:Option<(f32,Vec2)>=None;
    let mut suppress_tap=false;
    let mut message=String::from(
        "Choose MAIN or SCOUT. HOLD protects the pass. Press PLAY.");
    let mut floaters:Vec<Floater>=Vec::new();

    loop {
        let sw=screen_width();
        let sh=screen_height();
        let (area,squad_buttons,command_buttons,utility_buttons,wide)=
            board_layout(sw,sh);
        let view=world_view(camera,zoom,area);
        let touch_list=touches();
        let active:Vec<_>=touch_list.iter().filter(|t|
            !matches!(t.phase,TouchPhase::Ended|TouchPhase::Cancelled)).collect();
        let start=touch_list.iter().find(|t|t.phase==TouchPhase::Started)
            .map(|t|t.position)
            .or_else(|| if is_mouse_button_pressed(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            } else {None});
        let current=active.first().map(|t|t.position)
            .or_else(||if is_mouse_button_down(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            } else {None});
        let ended=touch_list.iter().find(|t|t.phase==TouchPhase::Ended)
            .map(|t|t.position)
            .or_else(||if is_mouse_button_released(MouseButton::Left) {
                let (x,y)=mouse_position();Some(vec2(x,y))
            }else{None});

        if active.len()>=2 {
            suppress_tap=true;
            gesture=None;
            let middle=(active[0].position+active[1].position)*0.5;
            let distance=active[0].position.distance(active[1].position);
            if let Some((old_distance,old_middle))=pinch_prev {
                pan(&mut camera,zoom,area,middle-old_middle);
                if old_distance>8. {
                    zoom_on(&mut camera,&mut zoom,distance/old_distance,
                        middle,area);
                }
            }
            pinch_prev=Some((distance,middle));
        } else {pinch_prev=None;}
        if active.is_empty() && suppress_tap {suppress_tap=false;}
        let (mx,my)=mouse_position();
        let wheel=mouse_wheel().1;
        if wheel.abs()>0.001 && area.contains(vec2(mx,my)) {
            zoom_on(&mut camera,&mut zoom,(1.+wheel*0.14).max(0.75),
                vec2(mx,my),area);
        }

        let restart=|seed:u64,layout_idx:usize|reset(seed,layouts[layout_idx]);
        if is_key_pressed(KeyCode::Space) {
            running=!running && !battle.ended;
        }
        if is_key_pressed(KeyCode::Key1) {chosen=0;}
        if is_key_pressed(KeyCode::Key2) {chosen=1;}
        if is_key_pressed(KeyCode::M) {mode=Order::Move;}
        if is_key_pressed(KeyCode::A) {mode=Order::Attack;}
        if is_key_pressed(KeyCode::H) {
            if battle.issue_player_order(chosen,Order::Hold,battle.units[chosen].pos) {
                message="HOLD: Entrenchment rises; fatigue recovers.".into();
            }
        }
        if is_key_pressed(KeyCode::R) {
            if battle.issue_player_order(chosen,Order::Retreat,battle.depots[0]) {
                message="RETREAT: Your soldiers are falling back to the depot.".into();
            }
        }
        if is_key_pressed(KeyCode::N) {
            seed+=1;
            layout_idx=(layout_idx+1)%layouts.len();
            battle=restart(seed,layout_idx);
            previous_positions=std::array::from_fn(|i|battle.units[i].pos);
            chosen=0;mode=Order::Hold;
            running=false;accumulator=0.;camera=vec2(7.,4.);zoom=1.;
            floaters.clear();
            message="New siege. Order your squads and press PLAY.".into();
        }

        if active.len()<2 && !suppress_tap {
            if let Some(p)=start {
                if let Some(i)=squad_buttons.iter().position(|r|r.contains(p)) {
                    chosen=i;
                    mode=if battle.units[i].order==Order::Hold {
                        Order::Hold
                    } else {Order::Move};
                    let u=battle.units[i];
                    let view=world_view(camera,zoom,area);
                    if (u.pos.0 as f32)<view.x+0.5 ||
                        (u.pos.0 as f32)>view.x+view.w-0.5 ||
                        (u.pos.1 as f32)<view.y+0.5 ||
                        (u.pos.1 as f32)>view.y+view.h-0.5 {
                        camera=vec2(u.pos.0 as f32,u.pos.1 as f32);
                        clamp_camera(&mut camera,zoom,area);
                    }
                    message=if i==0 {"MAIN selected — defend the crossing."}
                        else {"SCOUT selected — guard the rear or raid enemy supplies."}.into();
                } else if let Some(i)=command_buttons.iter().position(|r|r.contains(p)) {
                    let cmd=[Order::Move,Order::Attack,Order::Hold,Order::Retreat][i];
                    mode=cmd;
                    if cmd==Order::Hold || cmd==Order::Retreat {
                        let pos=battle.units[chosen].pos;
                        if battle.issue_player_order(chosen,cmd,pos) {
                            message=if cmd==Order::Hold {
                                "HOLD: Defense strengthens while soldiers recover."
                            } else {
                                "RETREAT: Return toward your supply depot."
                            }.into();
                        } else {
                            message="Squad cannot change orders while routing.".into();
                        }
                    } else {
                        message=format!("{} selected. TAP a map destination.",
                            cmd.label());
                    }
                } else if let Some(i)=utility_buttons.iter().position(|r|r.contains(p)) {
                    match i {
                        0=>{
                            running=!running && !battle.ended;
                            message=if running {"Battle running."}else{"Paused. Plan your orders."}.into();
                        }
                        1=>{fast=!fast;
                            message=if fast {"2X SPEED"}else{"NORMAL SPEED"}.into();}
                        2=>{
                            seed+=1;
                            layout_idx=(layout_idx+1)%layouts.len();
                            battle=restart(seed,layout_idx);
                            previous_positions=std::array::from_fn(|i|battle.units[i].pos);
                            chosen=0;mode=Order::Hold;
                            running=false;accumulator=0.;camera=vec2(7.,4.);zoom=1.;
                            floaters.clear();
                            message="New siege. Order squads, then press PLAY.".into();
                        }
                        3=>zoom_on(&mut camera,&mut zoom,1.25,area.center(),area),
                        4=>zoom_on(&mut camera,&mut zoom,0.8,area.center(),area),
                        _=>{}
                    }
                } else if minimap_rect(area).contains(p) {
                    let mini=minimap_rect(area);
                    camera=vec2(
                        (p.x-mini.x)/mini.w*FIELD_WIDTH as f32,
                        (p.y-mini.y)/mini.h*FIELD_HEIGHT as f32);
                    clamp_camera(&mut camera,zoom,area);
                    message="Camera moved. Tap a squad or give an order.".into();
                } else if area.contains(p) {
                    gesture=Some(Gesture{origin:p,last:p,dragging:false});
                }
            }
            if let (Some(g),Some(p))=(gesture.as_mut(),current) {
                if p.distance(g.origin)>11. {g.dragging=true;}
                if g.dragging {pan(&mut camera,zoom,area,p-g.last);}
                g.last=p;
            }
            if let Some(p)=ended {
                if let Some(g)=gesture.take() {
                    if !g.dragging && g.origin.distance(p)<11. && area.contains(p) {
                        let target=cell_from(p,area,world_view(camera,zoom,area));
                        if let Some(i)=(0..2).find(|&i|battle.units[i].troops>0
                            && battle.units[i].pos==target) {
                            chosen=i;
                            message=format!("{} selected.",
                                if i==0 {"MAIN"}else{"SCOUT"});
                        } else if matches!(mode,Order::Move|Order::Attack) {
                            if battle.issue_player_order(chosen,mode,target) {
                                message=format!("{} squad {} to ({},{}).",
                                    mode.label(),chosen+1,target.0,target.1);
                            } else {
                                message="Invalid destination or squad is routing.".into();
                            }
                        } else {
                            message="Choose MOVE or ATTACK, then tap a destination.".into();
                        }
                    }
                }
            }
        }

        let turn_duration=if fast {0.36_f32}else{0.72_f32};
        if running && !battle.ended {
            accumulator+=get_frame_time().min(0.12);
            for _ in 0..3 {
                if accumulator<turn_duration || battle.ended {break;}
                accumulator-=turn_duration;
                previous_positions=std::array::from_fn(|i|battle.units[i].pos);
                let losses_before=[
                    battle.stats[0].combat_lost,battle.stats[1].combat_lost,
                    battle.stats[0].attrition_lost,battle.stats[1].attrition_lost,
                ];
                let troops_before:[u32;4]=std::array::from_fn(|i|
                    battle.units[i].troops);
                battle.step_player();
                // Use ACTUAL squad casualties, not a team-wide loss
                // displayed above the wrong unit. The event is explanatory.
                for i in 0..4 {
                    let losses=troops_before[i].saturating_sub(battle.units[i].troops);
                    if losses==0 {continue;}
                    let u=battle.units[i];
                    floaters.push(Floater{
                        at:vec2(u.pos.0 as f32,u.pos.1 as f32),
                        message:format!("-{} {}",losses,
                            if i<2 {"BLUE"}else{"ENEMY"}),
                        born:get_time(),
                        color:if i<2 {ORANGE}else{GREEN},
                    });
                }
                if battle.stats[0].combat_lost>losses_before[0] {
                    let loss=battle.stats[0].combat_lost-losses_before[0];
                    message=format!("Your squads lost {} soldiers in combat.",loss);
                }
                if battle.stats[0].attrition_lost>losses_before[2] {
                    message="Your unsupplied soldiers are losing strength!".into();
                }
                if battle.ended {
                    running=false;
                    message=if battle.winner==Some(0) {
                        "VICTORY — Your battle plan worked.".into()
                    } else if battle.winner==Some(1) {
                        "DEFEAT — The enemy achieved its objective.".into()
                    } else {"DRAW — No decisive objective reached.".into()};
                }
            }
        }else{accumulator=0.;}
        floaters.retain(|f|get_time()-f.born<1.5);
        // Pausing shows actual authoritative positions immediately.
        let alpha=if battle.ended || !running {1.}
            else{(accumulator/turn_duration).clamp(0.,1.)};
        let visual_positions:[Vec2;4]=std::array::from_fn(|i|
            interpolated(previous_positions[i],battle.units[i].pos,alpha));
        clear_background(BG);
        draw_rectangle(0.,0.,sw,96.,rgba(15,29,42));
        text(if wide {"GENESIS / TACTICAL SIEGE"}
            else {"GENESIS / SIEGE"},13.,27.,18.,WHITE);
        text(&format!("ROUND {:02}/{:02}     {}",
            battle.round,MAX_ROUNDS,if battle.ended {"FINISHED"}else if running {"LIVE"}else{"PAUSED"}),
            13.,50.,13.,if running {GREEN}else{MUTED});
        text(&format!("YOU {}     ENEMY {}     PASS {}/{}",
            battle.remaining(0),battle.remaining(1),
            battle.pass_hold_rounds,MOUNTAIN_DEFEND_ROUNDS),
            13.,72.,13.,WHITE);
        let progress=(battle.pass_hold_rounds as f32/MOUNTAIN_DEFEND_ROUNDS as f32)
            .clamp(0.,1.);
        draw_rectangle(13.,83.,sw-26.,5.,rgba(42,62,70));
        draw_rectangle(13.,83.,(sw-26.)*progress,5.,GREEN);
        let view=world_view(camera,zoom,area);
        map_render(&battle,area,view,visual_positions,chosen,None,mode,&floaters);
        draw_minimap(&battle,area,view);

        let control_x=if wide {area.x+area.w+12.}else{10.};
        let info_y=if wide {112.}else{area.y+area.h+10.};
        if wide {
            text("DEFEND THE PASS",control_x,info_y,17.,GOLD);
            text("Main: HOLD until 24 rounds.",control_x,info_y+24.,13.,WHITE);
            text("Scout: Guard rear or raid.",control_x,info_y+42.,13.,MUTED);
        }else{
            text("HOLD 24 ROUNDS  /  PROTECT YOUR REAR",
                control_x,info_y+2.,(sw/29.).clamp(10.,15.),GOLD);
            text(&primary_state(&battle,chosen),control_x,info_y+20.,
                (sw/35.).clamp(9.,12.),WHITE);
        }
        for i in 0..2 {
            let u=battle.units[i];
            let name=if i==0 {"MAIN"}else{"SCOUT"};
            button(squad_buttons[i],
                &format!("{}  {} SOLDIERS",name,u.troops),i==chosen);
        }
        for (i,label) in ["MOVE","ATTACK","HOLD","RETREAT"].iter().enumerate() {
            button(command_buttons[i],label,
                mode==[Order::Move,Order::Attack,Order::Hold,Order::Retreat][i]);
        }
        for (i,label) in [
            if running {"PAUSE"}else{"PLAY"},
            if fast {"2X ON"}else{"1X"},
            "NEW MAP","+","-"
        ].iter().enumerate() {
            button(utility_buttons[i],label,
                i==0 && running || i==1 && fast);
        }
        let note_y=if wide {
            utility_buttons[0].y+55.
        }else{
            utility_buttons[0].y+58.
        };
        if wide {
            text("CURRENT SQUAD",control_x,utility_buttons[0].y+83.,13.,GOLD);
            let u=battle.units[chosen];
            for (j,item) in [
                format!("TROOPS: {}",u.troops),
                format!("MORALE: {}",u.state.morale),
                format!("FATIGUE: {}",u.state.fatigue),
                format!("FOOD: {}/6",u.state.provisions),
                format!("SUPPLY: {}",if battle.connected(chosen) {"CONNECTED"}else{"CUT OFF"}),
                format!("ORDER: {}",u.order.label()),
            ].iter().enumerate() {
                text(item,control_x,utility_buttons[0].y+110.+j as f32*25.,
                    15.,if j==4 && !battle.connected(chosen) {ORANGE}else{WHITE});
            }
        } else {
            text("TAP SQUAD -> ORDER -> MAP. DRAG TO PAN. PINCH TO ZOOM.",
                10.,note_y-19.,(sw/36.).clamp(9.,11.),MUTED);
        }
        let banner_y=if wide {(sh-63.).max(utility_buttons[0].y+270.)}
            else{(sh-24.).max(note_y)};
        let textmsg=if message.chars().count()>59 {
            format!("{}...",message.chars().take(56).collect::<String>())
        } else{message.clone()};
        let banner_x=if wide {control_x-2.0}else{0.0};
        draw_rectangle(banner_x,banner_y-23.0,sw-banner_x,28.0,
            rgba(19,36,48));
        text(&textmsg,if wide {control_x}else{10.0},banner_y-4.0,
            (sw/35.0).clamp(10.0,14.0),WHITE);

        if battle.ended {
            let box_w=(sw*0.86).min(390.);
            let box_h=190.;
            let rx=(sw-box_w)*0.5;
            let ry=(sh-box_h)*0.43;
            draw_rectangle(rx-5.,ry-5.,box_w+10.,box_h+10.,
                Color::new(0.025,0.05,0.075,0.95));
            draw_rectangle_lines(rx,ry,box_w,box_h,2.,
                if battle.winner==Some(0) {GREEN}else{ORANGE});
            let headline=match battle.winner {
                Some(0)=>"VICTORY",
                Some(1)=>"DEFEAT",
                _=>"DRAW",
            };
            center_text(headline,vec2(sw*0.5,ry+45.),30.,
                if battle.winner==Some(0) {GREEN}else{ORANGE});
            let cause=cause_label(battle.victory_reason);
            // Two lines to keep the cause legible on narrow screens.
            let short=if cause.len()>34 {
                format!("{}...",cause.chars().take(31).collect::<String>())
            }else{cause.into()};
            center_text(&short,vec2(sw*0.5,ry+93.),14.,WHITE);
            center_text(&format!("BLUE {}    ORANGE {}",
                battle.remaining(0),battle.remaining(1)),
                vec2(sw*0.5,ry+127.),16.,MUTED);
            center_text("Tap NEW MAP to play again",
                vec2(sw*0.5,ry+161.),14.,GOLD);
        }
        next_frame().await;
    }
}
