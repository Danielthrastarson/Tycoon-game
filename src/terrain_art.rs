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
            let mut rgb=palette(t);

            let sunlight=0.88+broad*0.23+mid*0.08+(micro-0.5)*0.075;
            for c in &mut rgb {*c*=sunlight;}

            match t {
                Terrain::Sea=>{
                    // Pale shallows beside land, darker water farther in.
                    // The brighter line is clearly NOT a traversable bridge.
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
                }
                Terrain::Mountain=>{
                    // Fine directional ridges and contour shadows produce
                    // relief without allocating a 3D geometry engine.
                    let ridge=(x*7.2+y*4.7+broad*6.3).sin();
                    let crack=(x*15.5-y*6.2+mid*5.7).sin().abs();
                    let value=ridge*11.0
                        +if crack>0.955 {16.0}else{0.0}
                        -(1.0-mid)*13.0;
                    rgb[0]+=value;
                    rgb[1]+=value+2.0;
                    rgb[2]+=value+4.0;
                }
                Terrain::Forest=>{
                    rgb[0]-=8.0;
                    rgb[1]+=mid*14.0;
                    rgb[2]+=broad*8.0;
                }
                Terrain::Plain | Terrain::Farm=>{
                    let glade=(x*5.4-y*2.3+broad*2.8).sin();
                    let shift=glade*4.5+micro*4.0;
                    rgb[0]+=shift+2.0;
                    rgb[1]+=shift;
                    rgb[2]+=shift*0.56;
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
            assert!(crossing[0]>crossing[2]-12,
                "physical crossing must not look like deep water");
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
