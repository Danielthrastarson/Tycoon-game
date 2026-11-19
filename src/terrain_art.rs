//! Genesis V0.4.5a: cached, deterministic painterly landscape renderer.
//!
//! This module translates the authoritative 15x9 terrain grid into a
//! high-resolution RGBA atlas ONCE when the world is generated. It never
//! changes walkability, supply, combat, land ownership or world RNG.
//! Forest canopy, coast gradients, stone contours and shallow-water glints
//! are visual-only. The camera samples a region of this atlas with linear
//! filtering instead of drawing hundreds of colored squares every frame.
use crate::frontier::Terrain;
use crate::frontier_combat::{Battle, FIELD_HEIGHT, FIELD_WIDTH};

pub const DEFAULT_PIXELS_PER_CELL:u16=112;

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct TerrainAtlas {
    pub width:u16,
    pub height:u16,
    pub rgba:Vec<u8>,
}

#[inline]
fn mix(a:f32,b:f32,t:f32)->f32 {a+(b-a)*t}
#[inline]
fn saturate(a:f32)->f32 {a.clamp(0.0,1.0)}
#[inline]
fn smooth(a:f32)->f32 {a*a*(3.0-2.0*a)}
#[inline]
fn h32(mut x:u32)->u32 {
    x ^= x>>16;
    x=x.wrapping_mul(0x7feb352d);
    x ^= x>>15;
    x=x.wrapping_mul(0x846ca68b);
    x ^ (x>>16)
}
#[inline]
fn rand01(seed:u32,x:i32,y:i32)->f32 {
    let n=h32(seed ^ (x as u32).wrapping_mul(0x9e3779b9)
        ^ (y as u32).wrapping_mul(0x85ebca6b));
    (n&0xffff) as f32/65535.0
}
#[inline]
fn value_noise(x:f32,y:f32,seed:u32)->f32 {
    let ix=x.floor() as i32;
    let iy=y.floor() as i32;
    let a=smooth(x-ix as f32);
    let b=smooth(y-iy as f32);
    let low=mix(rand01(seed,ix,iy),rand01(seed,ix+1,iy),a);
    let high=mix(rand01(seed,ix,iy+1),rand01(seed,ix+1,iy+1),a);
    mix(low,high,b)
}

#[inline]
fn material(b:&Battle,x:f32,y:f32)->Terrain {
    let bx=(x.floor() as i16).clamp(0,FIELD_WIDTH-1);
    let by=(y.floor() as i16).clamp(0,FIELD_HEIGHT-1);
    b.terrain_at((bx,by))
}

fn palette(t:Terrain)->[f32;3] {
    match t {
        Terrain::Plain=>[103.0,143.0,112.0],
        Terrain::Farm=>[151.0,159.0,100.0],
        Terrain::Forest=>[54.0,111.0,89.0],
        Terrain::Mountain=>[134.0,150.0,149.0],
        Terrain::Sea=>[34.0,105.0,135.0],
    }
}

// Smooth geographic influence masks are sampled at a tenth-cell scale,
// rather than using hard rectangular forest/mountain stamps. Adjacent tiles
// form continuous landforms; isolated features taper into their surroundings.
const FIELD_DETAIL:usize=12;
struct LandscapeFields {
    width:usize,
    forest:Vec<f32>,
    relief:Vec<f32>,
}
impl LandscapeFields {
    fn new(battle:&Battle)->Self {
        let width=FIELD_WIDTH as usize*FIELD_DETAIL+1;
        let height=FIELD_HEIGHT as usize*FIELD_DETAIL+1;
        let mut forest=vec![0.0; width*height];
        let mut relief=vec![0.0; width*height];
        for iy in 0..height {
            let y=iy as f32/FIELD_DETAIL as f32;
            for ix in 0..width {
                let x=ix as f32/FIELD_DETAIL as f32;
                let ix0=x.floor() as i16;
                let iy0=y.floor() as i16;
                let mut trees=0.0;
                let mut rocks=0.0;
                for by in (iy0-2).max(0)..=(iy0+2).min(FIELD_HEIGHT-1) {
                    for bx in (ix0-2).max(0)..=(ix0+2).min(FIELD_WIDTH-1) {
                        let dx=x-(bx as f32+0.5);
                        let dy=y-(by as f32+0.5);
                        let d2=dx*dx+dy*dy;
                        match battle.terrain_at((bx,by)) {
                            Terrain::Forest=>{
                                let influence=(1.0-d2/1.08_f32.powi(2)).max(0.0);
                                trees+=influence*influence*0.93;
                            }
                            Terrain::Mountain=>{
                                let influence=(1.0-d2/1.45_f32.powi(2)).max(0.0);
                                rocks+=influence*influence*0.86;
                            }
                            _=>{}
                        }
                    }
                }
                forest[iy*width+ix]=trees.min(1.0);
                relief[iy*width+ix]=rocks.min(1.0);
            }
        }
        Self {width,forest,relief}
    }

    #[inline]
    fn interpolated(&self,data:&[f32],x:f32,y:f32)->f32 {
        let xf=x.max(0.0)*FIELD_DETAIL as f32;
        let yf=y.max(0.0)*FIELD_DETAIL as f32;
        let ix=(xf.floor() as usize).min(self.width-2);
        let iy=(yf.floor() as usize).min(data.len()/self.width-2);
        let ax=(xf-ix as f32).clamp(0.0,1.0);
        let ay=(yf-iy as f32).clamp(0.0,1.0);
        let top=mix(data[iy*self.width+ix],data[iy*self.width+ix+1],ax);
        let bottom=mix(data[(iy+1)*self.width+ix],
            data[(iy+1)*self.width+ix+1],ax);
        mix(top,bottom,ay)
    }
    #[inline]
    fn sample(&self,x:f32,y:f32)->(f32,f32) {
        (self.interpolated(&self.forest,x,y),
         self.interpolated(&self.relief,x,y))
    }
}

#[inline]
fn blend_into(pixels:&mut [u8],w:usize,x:usize,y:usize,
    rgb:[f32;3],opacity:f32) {
    let at=(y*w+x)*4;
    for c in 0..3 {
        pixels[at+c]=mix(pixels[at+c] as f32,rgb[c],opacity)
            .clamp(0.0,255.0) as u8;
    }
}

/// Paints a soft-edged canopy circle onto an already completed atlas.
/// Pixel loops are clipped to the owning forest tile: decorations never
/// visually turn physical water or mountain crossings into obstacles.
fn canopy_circle(pixels:&mut [u8],width:usize,tile:u16,
    tx:i16,ty:i16,cx:f32,cy:f32,r:f32,light:f32) {
    let tile=tile as usize;
    let x0=(tx as usize)*tile;
    let y0=(ty as usize)*tile;
    let left=((cx-r-1.0).floor() as i32).max(x0 as i32) as usize;
    let top=((cy-r-1.0).floor() as i32).max(y0 as i32) as usize;
    let right=((cx+r+1.0).ceil() as i32).min((x0+tile) as i32) as usize;
    let bottom=((cy+r+1.0).ceil() as i32).min((y0+tile) as i32) as usize;
    for y in top..bottom {
        for x in left..right {
            let dx=x as f32+0.5-cx;
            let dy=y as f32+0.5-cy;
            let d=(dx*dx+dy*dy).sqrt();
            let coverage=saturate(r+0.8-d);
            if coverage==0.0 {continue;}
            // Sun from north-west, with a dark south-east shadow.
            let direction=(-dx-dy)/(r*2.0);
            let value=(light+direction*18.0).clamp(-25.0,28.0);
            let rgb=[(34.0+value*0.62).max(10.0),
                (100.0+value).max(20.0),
                (71.0+value*0.69).max(15.0)];
            blend_into(pixels,width,x,y,rgb,coverage*0.66);
        }
    }
}

/// Stable image generation. Every atlas sample comes from the authoritative
/// map and fixed visual seed. Terrain edges may be visually softened by up
/// to about a tenth of a cell; pathfinding and passability are unchanged.
pub fn build_atlas(battle:&Battle,tile_pixels:u16)->TerrainAtlas {
    assert!((8..=192).contains(&tile_pixels));
    let width=FIELD_WIDTH as u16*tile_pixels;
    let height=FIELD_HEIGHT as u16*tile_pixels;
    let w=width as usize;
    let h=height as usize;
    let mut rgba=vec![255u8;w*h*4];
    let seed=(battle.seed as u32)
        ^ ((battle.seed>>32) as u32).rotate_left(7)
        ^ (battle.layout as u32).wrapping_mul(0x9e3779b9);
    let fields=LandscapeFields::new(battle);

    for py in 0..h {
        let y=(py as f32+0.5)/tile_pixels as f32;
        for px in 0..w {
            let x=(px as f32+0.5)/tile_pixels as f32;
            let broad=value_noise(x*1.15,y*1.15,seed^0x6b12a9c3);
            let mid=value_noise(x*4.6,y*4.6,seed^0x2ce7110f);
            let micro=value_noise(x*21.0,y*21.0,seed^0xda915b77);

            // The category boundary follows a subtle, seeded curve.
            // Solid physical cell centers cannot change material.
            let bend=(broad-0.5)*0.17+(mid-0.5)*0.045;
            let warped_x=x+bend;
            let warped_y=y+(mid-0.5)*0.12+(broad-0.5)*0.05;
            let t=material(battle,warped_x,warped_y);
            let (trees,relief)=fields.sample(warped_x,warped_y);
            let sunlight=0.88+broad*0.23+mid*0.08+(micro-0.5)*0.075;
            let mut rgb=palette(Terrain::Plain);
            for c in &mut rgb {*c*=sunlight;}

            if t==Terrain::Sea {
                rgb=palette(Terrain::Sea);
                for c in &mut rgb {*c*=sunlight;}
                // Pale shallows beside solid land, darker water farther in.
                let mut land_near=0.0_f32;
                for (dx,dy) in [(0.105,0.0),(-0.105,0.0),
                                (0.0,0.105),(0.0,-0.105)] {
                    if material(battle,warped_x+dx,warped_y+dy)!=Terrain::Sea {
                        land_near=1.0;
                    }
                }
                let shimmer=(x*13.8+y*4.1+mid*3.5).sin();
                rgb[0]+=land_near*27.0+shimmer.max(0.0)*3.8;
                rgb[1]+=land_near*29.0+shimmer.max(0.0)*5.0;
                rgb[2]+=land_near*18.0+shimmer.max(0.0)*5.5;
            } else {
                let glade=(x*5.4-y*2.3+broad*2.8).sin();
                let ground=glade*4.5+micro*4.0;
                rgb[0]+=ground+2.0;
                rgb[1]+=ground;
                rgb[2]+=ground*0.56;

                if t==Terrain::Farm {
                    let farm=palette(Terrain::Farm);
                    for c in 0..3 {rgb[c]=mix(rgb[c],farm[c]*sunlight,0.74);}
                }

                // Woodland extends beyond its original cell boundary.
                // Organic canopy edges remove the old green rectangles.
                let forest_strength=(trees*(0.91+(mid-0.5)*0.22))
                    .clamp(0.0,1.0);
                let forest_color=[
                    48.0*sunlight+mid*9.0,
                    105.0*sunlight+mid*14.0,
                    77.0*sunlight+broad*9.0,
                ];
                for c in 0..3 {
                    rgb[c]=mix(rgb[c],forest_color[c],forest_strength);
                }

                // Relief flows across whole ridges, including the approach
                // slopes. Northwest-facing slopes are warm and bright,
                // southeast-facing slopes are cool and shadowed.
                if relief>0.02 {
                    let (_,sunward)=fields.sample(warped_x-0.075,warped_y-0.075);
                    let (_,shadowward)=fields.sample(warped_x+0.075,warped_y+0.075);
                    let directional=(sunward-shadowward)*64.0;
                    let ridge=(x*7.2+y*4.7+broad*6.3).sin();
                    let crack=(x*15.5-y*6.2+mid*5.7).sin().abs();
                    let outline=if crack>0.964 {12.0}else{0.0};
                    let lighting=directional+ridge*10.0-(1.0-mid)*9.0+outline;
                    let rock=[
                        133.0*sunlight+lighting,
                        146.0*sunlight+lighting+2.0,
                        142.0*sunlight+lighting+3.0,
                    ];
                    let stone_strength=(relief*(1.05+(mid-0.5)*0.18))
                        .clamp(0.0,1.0);
                    for c in 0..3 {
                        rgb[c]=mix(rgb[c],rock[c],stone_strength);
                    }
                    // Thin natural topographic contours, visible mostly
                    // on slopes rather than as a repeated tile icon.
                    let band=(relief*7.2+broad*0.48).fract();
                    if stone_strength>0.16 && band<0.055 {
                        for c in &mut rgb {*c-=stone_strength*10.0;}
                    }
                }
            }

            // On land, a thin warm coastal bank helps the river look inset
            // rather than like an opaque line of painted square tiles.
            if t!=Terrain::Sea {
                let mut coast=false;
                for (dx,dy) in [(0.08,0.0),(-0.08,0.0),
                                (0.0,0.08),(0.0,-0.08)] {
                    if material(battle,warped_x+dx,warped_y+dy)==Terrain::Sea {
                        coast=true;
                    }
                }
                if coast {
                    rgb[0]=mix(rgb[0],171.0,0.37);
                    rgb[1]=mix(rgb[1],168.0,0.31);
                    rgb[2]=mix(rgb[2],131.0,0.26);
                }
            }
            let at=(py*w+px)*4;
            for c in 0..3 {rgba[at+c]=rgb[c].clamp(0.0,255.0) as u8;}
        }
    }

    // Clustered forest crowns form a clear visual mass when zoomed in.
    // Stable visual RNG; no runtime tree animations or additional GPU draw
    // calls. Dense tree drawing never alters a terrain or pathfinding cell.
    for cy in 0..FIELD_HEIGHT {
        for cx in 0..FIELD_WIDTH {
            if battle.terrain_at((cx,cy))!=Terrain::Forest {continue;}
            for n in 0..9 {
                let key=seed ^ (n as u32).wrapping_mul(0x7f4a7c15);
                let tx=(0.17+rand01(key,cx as i32,cy as i32)*0.66)
                    *tile_pixels as f32;
                let ty=(0.17+rand01(key^0x91ef0013,cx as i32,cy as i32)*0.66)
                    *tile_pixels as f32;
                let radius=(0.045+rand01(key^0xcad31527,cx as i32,cy as i32)
                    *0.044)*tile_pixels as f32;
                let x=cx as f32*tile_pixels as f32+tx;
                let y=cy as f32*tile_pixels as f32+ty;
                let light=rand01(key^0x3f18909b,cx as i32,cy as i32)*26.0-9.0;
                canopy_circle(&mut rgba,w,tile_pixels,cx,cy,x+1.8,y+2.7,
                    radius*1.08,light-13.0);
                canopy_circle(&mut rgba,w,tile_pixels,cx,cy,x,y,
                    radius,light+4.0);
            }
        }
    }
    TerrainAtlas{width,height,rgba}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier_combat::{CombatRules,Layout,Policy,Scenario};

    fn battle(seed:u64,layout:Layout,rules:CombatRules)->Battle {
        Battle::new(seed,Scenario::MountainPass,Policy::Adaptive,
            false,rules).with_layout(layout)
    }

    #[test]
    fn images_are_deterministic_opaque_and_correctly_sized() {
        for layout in Layout::ALL {
            let b=battle(1005,layout,CombatRules::FULL);
            let first=build_atlas(&b,16);
            let second=build_atlas(&b,16);
            assert_eq!(first,second);
            assert_eq!(first.width,FIELD_WIDTH as u16*16);
            assert_eq!(first.height,FIELD_HEIGHT as u16*16);
            assert_eq!(first.rgba.len(),first.width as usize*
                first.height as usize*4);
            assert!(first.rgba.chunks_exact(4).all(|px|px[3]==255));
        }
    }

    #[test]
    fn image_cannot_change_simulation_and_ignores_combat_ablation() {
        for layout in Layout::ALL {
            let before=battle(1022,layout,CombatRules::FULL);
            let all_off=battle(1022,layout,CombatRules::from_mask(0));
            let painted=build_atlas(&before,12);
            assert_eq!(before,battle(1022,layout,CombatRules::FULL));
            assert_eq!(painted,build_atlas(&all_off,12));
        }
    }

    fn pixel(atlas:&TerrainAtlas,x:usize,y:usize)->[u8;3] {
        let at=(y*atlas.width as usize+x)*4;
        [atlas.rgba[at],atlas.rgba[at+1],atlas.rgba[at+2]]
    }

    #[test]
    fn water_and_pass_centers_remain_visually_distinguishable() {
        for layout in [Layout::RiverFord,Layout::SplitValley] {
            let b=battle(1001,layout,CombatRules::FULL);
            let art=build_atlas(&b,32);
            let px=|xy:(i16,i16)|pixel(&art,
                (xy.0 as usize)*32+16,(xy.1 as usize)*32+16);
            let water=(0..FIELD_HEIGHT)
                .flat_map(|y|(0..FIELD_WIDTH).map(move|x|(x,y)))
                .find(|&p|b.terrain_at(p)==Terrain::Sea)
                .expect("a river contains a real impassable water tile");
            let sea=px(water);
            let crossing=px(b.objective);
            assert!(sea[2]>sea[0]+35,
                "water must retain obvious blue chroma");
            assert!(crossing[0] as i32 > sea[0] as i32+45,
                "traversable mountain crossing must be much less blue than deep water");
        }
    }

    #[test]
    fn generated_landscape_varies_with_seed_without_changing_paths() {
        let a=battle(1001,Layout::Ridge,CombatRules::FULL);
        let b=battle(1002,Layout::Ridge,CombatRules::FULL);
        let ta=build_atlas(&a,12);
        let tb=build_atlas(&b,12);
        assert_ne!(ta.rgba,tb.rgba);
        assert!(a.audit()&&b.audit());
    }
}
