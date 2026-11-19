//! GENESIS FRONTIER — deterministic strategy rules, independent of rendering.
//! First playable competitive prototype: one player and three autonomous rivals.
//! This is NOT networked multiplayer yet. Client commands are explicit and can
//! later be validated by a single authoritative multiplayer server.

use std::collections::VecDeque;

pub const WIDTH: usize = 72;
pub const HEIGHT: usize = 52;
pub const FACTIONS: usize = 4;
pub const NEUTRAL: u8 = 255;
const N: usize = WIDTH * HEIGHT;

/// A legible near-term objective for the first invasion; not a hardcoded win condition.
pub const NORTH_CROSSING: (i16,i16) = (39,16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain { Sea, Plain, Farm, Forest, Mountain }

impl Terrain {
    pub fn traversable(self) -> bool { self != Self::Sea }
    pub fn yield_food(self) -> i32 {
        match self { Self::Sea => 0, Self::Plain => 2, Self::Farm => 8,
            Self::Forest => 3, Self::Mountain => 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tile { pub terrain: Terrain, pub owner: u8 }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order { Move, Hold, Attack, Retreat }
impl Order {
    pub fn label(self) -> &'static str {
        match self { Self::Move => "MOVE", Self::Hold => "HOLD",
            Self::Attack => "ATTACK", Self::Retreat => "RETREAT" }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Kingdom {
    pub name: &'static str,
    pub capital: (i16,i16),
    pub food: i32,
    pub population: u32,
    pub alive: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Army {
    pub owner: u8,
    pub x: i16,
    pub y: i16,
    pub target: (i16,i16),
    pub order: Order,
    pub troops: u32,
    pub id: u32,
}

/// Reports observable consequences of one deterministic simulation tick.
/// The renderer reads this but cannot change world rules.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TurnReport {
    pub captured: [u32; FACTIONS],
    pub casualties: [u32; FACTIONS],
    pub clashes: u32,
    pub capital_taken: Option<(u8,u8)>, // winner, defeated
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    pub seed: u64,
    pub tick: u64,
    pub tiles: Vec<Tile>,
    pub kingdoms: [Kingdom; FACTIONS],
    pub armies: Vec<Army>,
    pub winner: Option<u8>,
    pub last_report: TurnReport,
    next_id: u32,
}

fn random_hash(seed: u64, x: i32, y: i32) -> u32 {
    let mut z = seed
        ^ (x as i64 as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as i64 as u64).wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 30;
    z = z.wrapping_mul(0xBF58476D1CE4E5B9);
    z ^= z >> 27;
    z = z.wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    (z >> 32) as u32
}

fn noise(seed: u64, x: usize, y: usize, cell: usize) -> u32 {
    let gx = (x / cell) as i32;
    let gy = (y / cell) as i32;
    let fx = (x % cell) as u32;
    let fy = (y % cell) as u32;
    let c = cell as u32;
    let a = random_hash(seed,gx,gy) % 256;
    let b = random_hash(seed,gx+1,gy) % 256;
    let d = random_hash(seed,gx,gy+1) % 256;
    let e = random_hash(seed,gx+1,gy+1) % 256;
    let top = (a*(c-fx) + b*fx) / c;
    let bottom = (d*(c-fx) + e*fx) / c;
    (top*(c-fy) + bottom*fy) / c
}

const CAPITALS: [(i16,i16);4] = [(11,11),(60,11),(11,40),(60,40)];
const NAMES: [&str;4] = ["AZURE", "EMBER", "VERDANT", "VIOLET"];

fn dist(a:(i16,i16), b:(i16,i16)) -> i16 { (a.0-b.0).abs() + (a.1-b.1).abs() }

impl World {
    pub fn index(x: i16, y:i16) -> usize { y as usize * WIDTH + x as usize }
    pub fn valid(x:i16,y:i16)->bool {
        x>=0 && y>=0 && (x as usize)<WIDTH && (y as usize)<HEIGHT
    }

    pub fn new(seed:u64)->Self {
        let mut tiles=Vec::with_capacity(N);
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let elevation = noise(seed ^ 0x471A, x,y, 12);
                let moisture = noise(seed ^ 0xCC55, x,y, 9);
                let mut terrain=if elevation < 44 { Terrain::Sea }
                    else if elevation > 200 { Terrain::Mountain }
                    else if moisture > 165 && elevation < 168 { Terrain::Forest }
                    else if moisture > 105 && elevation < 155 { Terrain::Farm }
                    else { Terrain::Plain };
                // One navigable river with two crossings creates meaningful
                // geography, without making the factions unreachable.
                let bend = ((y as i32 / 8 * 11 + 3) % 9) - 4;
                let rx = 36 + bend;
                let bridge = (15..=18).contains(&y) || (35..=38).contains(&y);
                if ((x as i32-rx).abs() <= 1) && !bridge { terrain=Terrain::Sea; }
                for cap in CAPITALS {
                    if dist((x as i16,y as i16),cap) <= 4 {
                        terrain=if (x+y)%4==0 {Terrain::Farm} else {Terrain::Plain};
                    }
                }
                tiles.push(Tile{terrain,owner:NEUTRAL});
            }
        }
        let kingdoms=std::array::from_fn(|i|Kingdom{
            name:NAMES[i],capital:CAPITALS[i],food:4000,
            population:2400,alive:true,
        });
        let mut world=Self{seed,tick:0,tiles,kingdoms,armies:Vec::new(),
            winner:None,last_report:TurnReport::default(),next_id:1};
        for i in 0..FACTIONS {
            let cap=CAPITALS[i];
            for dy in -7..=7i16 {
                for dx in -7..=7i16 {
                    let (x,y)=(cap.0+dx,cap.1+dy);
                    if Self::valid(x,y) && dx.abs()+dy.abs() <= 14 {
                        let tile=&mut world.tiles[Self::index(x,y)];
                        if tile.terrain.traversable() { tile.owner=i as u8; }
                    }
                }
            }
            // An immediately selectable frontier army makes the first
            // invasion seconds away, rather than a lengthy empty march.
            let toward_center = if cap.0 < WIDTH as i16 / 2 {12} else {-12};
            let forward = world.nearest_land((cap.0+toward_center,cap.1));
            world.armies.push(Army{owner:i as u8,x:forward.0,y:forward.1,
                target:forward,order:Order::Hold,troops:300,id:world.next_id});
            world.next_id+=1;
            // A small garrison gives defenders a real reason to attack
            // capitals rather than instantly ending the match.
            world.armies.push(Army{owner:i as u8,x:cap.0,y:cap.1,
                target:cap,order:Order::Hold,troops:150,id:world.next_id});
            world.next_id+=1;
        }
        world
    }

    pub fn winter(&self)->bool { (self.tick / 320) % 2 == 1 }

    pub fn terrain_at(&self,x:i16,y:i16)->Terrain {
        self.tiles[Self::index(x,y)].terrain
    }

    pub fn owner_at(&self,x:i16,y:i16)->u8 {
        self.tiles[Self::index(x,y)].owner
    }

    pub fn counts(&self)->[usize;FACTIONS] {
        let mut result=[0;FACTIONS];
        for t in &self.tiles {
            if t.owner != NEUTRAL { result[t.owner as usize]+=1; }
        }
        result
    }

    pub fn supply_connected(&self, who:u8, position:(i16,i16))->bool {
        let cap=self.kingdoms[who as usize].capital;
        if self.kingdoms[who as usize].food < 10 ||
            self.owner_at(cap.0,cap.1)!=who {return false;}
        let mut visited=vec![false;N];
        let mut queue=VecDeque::new();
        visited[Self::index(cap.0,cap.1)]=true;
        queue.push_back(cap);
        while let Some((x,y))=queue.pop_front() {
            if dist((x,y),position)<=2 {return true;}
            for (dx,dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                let (nx,ny)=(x+dx,y+dy);
                if !Self::valid(nx,ny) {continue;}
                let idx=Self::index(nx,ny);
                if !visited[idx] && self.tiles[idx].owner==who {
                    visited[idx]=true;
                    queue.push_back((nx,ny));
                }
            }
        }
        false
    }

    fn nearest_land(&self,pos:(i16,i16))->(i16,i16) {
        if Self::valid(pos.0,pos.1) && self.terrain_at(pos.0,pos.1).traversable() {
            return pos;
        }
        let mut best=None;
        for y in 0..HEIGHT as i16 {
            for x in 0..WIDTH as i16 {
                if !self.terrain_at(x,y).traversable(){continue;}
                let d=dist((x,y),pos);
                if best.is_none_or(|(bd,_,_)|d<bd) {best=Some((d,x,y));}
            }
        }
        best.map(|(_,x,y)|(x,y)).unwrap_or(pos)
    }

    pub fn issue(&mut self,who:u8,army_id:u32,order:Order,target:(i16,i16))->bool {
        if self.winner.is_some() || who as usize>=FACTIONS || !self.kingdoms[who as usize].alive {
            return false;
        }
        let target=if order==Order::Retreat {
            self.kingdoms[who as usize].capital
        } else { self.nearest_land(target) };
        let Some(army)=self.armies.iter_mut().find(|a|a.id==army_id && a.owner==who && a.troops>0) else {
            return false;
        };
        army.target=if order==Order::Hold { (army.x,army.y) } else {target};
        army.order=order;
        true
    }

    pub fn recruit(&mut self,who:u8)->Option<u32> {
        if self.winner.is_some() || who as usize >=FACTIONS {return None;}
        let k=&mut self.kingdoms[who as usize];
        if !k.alive || k.food<450 || k.population<550 ||
            self.armies.iter().filter(|a|a.owner==who && a.troops>0).count()>=4 {return None;}
        k.food-=450;
        k.population-=180;
        let id=self.next_id;self.next_id+=1;
        let cap=k.capital;
        self.armies.push(Army{owner:who,x:cap.0,y:cap.1,target:cap,
            order:Order::Hold,troops:165,id});
        Some(id)
    }

    fn next_step(&self,from:(i16,i16),to:(i16,i16))->Option<(i16,i16)> {
        if from==to {return None;}
        let to=self.nearest_land(to);
        let mut parents=vec![usize::MAX;N];
        let mut queue=VecDeque::new();
        let start=Self::index(from.0,from.1);
        let end=Self::index(to.0,to.1);
        parents[start]=start;
        queue.push_back(from);
        while let Some((x,y))=queue.pop_front() {
            if (x,y)==to {break;}
            for (dx,dy) in [(1,0),(-1,0),(0,1),(0,-1)] {
                let (nx,ny)=(x+dx,y+dy);
                if !Self::valid(nx,ny) || !self.terrain_at(nx,ny).traversable(){continue;}
                let ix=Self::index(nx,ny);
                if parents[ix]!=usize::MAX {continue;}
                parents[ix]=Self::index(x,y);
                queue.push_back((nx,ny));
            }
        }
        if parents[end]==usize::MAX {return None;}
        let mut cursor=end;
        while parents[cursor]!=start {
            cursor=parents[cursor];
            if cursor==start || cursor==usize::MAX {return None;}
        }
        Some(((cursor%WIDTH) as i16,(cursor/WIDTH) as i16))
    }

    fn advance_armies(&mut self) {
        for i in 0..self.armies.len() {
            let a=self.armies[i].clone();
            if a.troops==0 || !self.kingdoms[a.owner as usize].alive {continue;}
            if a.order==Order::Hold {continue;}
            if a.target==(a.x,a.y) {
                self.armies[i].order=Order::Hold;
                continue;
            }
            if self.tick%3!=0 {continue;}
            let Some((nx,ny))=self.next_step((a.x,a.y),a.target) else {continue;};
            if self.terrain_at(nx,ny)==Terrain::Mountain && self.tick%6!=0 {continue;}
            self.armies[i].x=nx;
            self.armies[i].y=ny;
        }
    }

    fn fight(&mut self) {
        // Simultaneous close-range damage avoids order-of-update advantages.
        let mut losses=vec![0u32;self.armies.len()];
        for i in 0..self.armies.len() {
            if self.armies[i].troops==0 {continue;}
            for j in (i+1)..self.armies.len() {
                let a=&self.armies[i];
                let b=&self.armies[j];
                if a.owner==b.owner || b.troops==0 || dist((a.x,a.y),(b.x,b.y))>1 {continue;}
                let sa=self.supply_connected(a.owner,(a.x,a.y));
                let sb=self.supply_connected(b.owner,(b.x,b.y));
                // A fortified or supplied army inflicts more damage.
                let defense_a=if self.terrain_at(a.x,a.y)==Terrain::Mountain {2} else {0};
                let defense_b=if self.terrain_at(b.x,b.y)==Terrain::Mountain {2} else {0};
                self.last_report.clashes+=1;
                losses[i]=losses[i].saturating_add((b.troops/22).max(2)
                    .saturating_sub(defense_a).saturating_sub(if sa{1}else{0}));
                losses[j]=losses[j].saturating_add((a.troops/22).max(2)
                    .saturating_sub(defense_b).saturating_sub(if sb{1}else{0}));
            }
        }
        for (a,d) in self.armies.iter_mut().zip(losses) {
            let actual=d.min(a.troops);
            a.troops-=actual;
            self.last_report.casualties[a.owner as usize]+=actual;
        }
    }

    fn claim_land(&mut self) {
        // Occupation is physical and localized. Remote magical ownership is not.
        // No army claims if a stronger enemy army is at its location.
        let actions:Vec<_>=self.armies.iter().filter(|a|a.troops>=25 &&
            self.kingdoms[a.owner as usize].alive).map(|a|(a.owner,a.x,a.y,a.troops))
            .collect();
        for (owner,x,y,troops) in actions {
            let hostile=self.armies.iter().any(|a|a.owner!=owner &&
                a.troops>=troops/2 && dist((a.x,a.y),(x,y))<=1);
            if hostile {continue;}
            // Nearby connected terrain creates narrow observable front lines.
            for (dx,dy) in [(0,0),(1,0),(-1,0),(0,1),(0,-1)] {
                let (px,py)=(x+dx,y+dy);
                if !Self::valid(px,py) {continue;}
                let index=Self::index(px,py);
                let tile=self.tiles[index];
                if !tile.terrain.traversable() || tile.owner==owner {continue;}
                // An army can extend its front while supplied; isolated troops
                // must reconnect their logistical corridor.
                if !self.supply_connected(owner,(x,y)){continue;}
                if tile.owner!=NEUTRAL && troops<65 {continue;}
                if let Some(defender)=self.kingdoms.iter().position(|k|k.capital==(px,py) &&
                    k.alive) {
                    if defender!=owner as usize {
                        let garrison=self.armies.iter().any(|a|a.owner==defender as u8 &&
                            a.troops>=55 && dist((a.x,a.y),(px,py))<=2);
                        if garrison || troops<110 {continue;}
                    }
                }
                self.tiles[index].owner=owner;
                self.last_report.captured[owner as usize]+=1;
                if let Some(defender)=self.kingdoms.iter().position(|k|k.capital==(px,py) &&
                    k.alive && k.capital!=self.kingdoms[owner as usize].capital) {
                    self.eliminate(defender,owner);
                }
            }
        }
    }

    fn eliminate(&mut self,defender:usize,winner:u8) {
        self.last_report.capital_taken=Some((winner,defender as u8));
        self.kingdoms[defender].alive=false;
        for tile in &mut self.tiles {
            if tile.owner==defender as u8 {tile.owner=winner;}
        }
        for a in &mut self.armies {
            if a.owner==defender as u8 {a.troops=0;}
        }
    }

    fn economy(&mut self) {
        let mut incomes=[0i32;FACTIONS];
        for tile in &self.tiles {
            if tile.owner!=NEUTRAL {
                incomes[tile.owner as usize]+=tile.terrain.yield_food();
            }
        }
        let winter=self.winter();
        for (i,k) in self.kingdoms.iter_mut().enumerate() {
            if !k.alive {continue;}
            let harvest=if winter {incomes[i]/5} else {incomes[i]};
            let army_cost: i32=self.armies.iter().filter(|a|a.owner==i as u8)
                .map(|a|(a.troops/8) as i32).sum();
            let upkeep=(k.population/17) as i32 + army_cost;
            k.food=(k.food+harvest-upkeep).clamp(0,30000);
            if k.food>1000 && k.population<25000 {
                k.population+=((k.population/150).max(2)).min(60);
            } else if k.food<200 {
                k.population=k.population.saturating_sub((k.population/38).max(1));
            }
        }
    }

    fn attrition(&mut self) {
        // Stranded and starving armies decline. The simulation does not
        // teleport supply or silently replenish soldiers.
        let losses:Vec<u32>=self.armies.iter().map(|a| {
            if a.troops==0{return 0;}
            let supply=self.supply_connected(a.owner,(a.x,a.y));
            let winter_stress=self.winter() &&
                self.terrain_at(a.x,a.y)==Terrain::Mountain;
            if supply && !winter_stress {0}
            else {(a.troops/28).max(1) + if winter_stress{2}else{0}}
        }).collect();
        for (a,lost) in self.armies.iter_mut().zip(losses) {
            a.troops=a.troops.saturating_sub(lost);
        }
    }

    fn bot_orders(&mut self) {
        // Bots use the same commands as the human. No teleportation or cheating.
        for who in 1..FACTIONS {
            if !self.kingdoms[who].alive {continue;}
            if self.tick%90==0 {
                let _=self.recruit(who as u8);
            }
            // Favor opponents across the map's main front rather than
            // spending the opening minute marching within one side.
            let own=self.kingdoms[who].capital;
            let target=self.kingdoms.iter().enumerate()
                .filter(|(i,k)|*i!=who && k.alive)
                .min_by_key(|(_,k)| {
                    let d=dist(own,k.capital);
                    let vertical=(own.1-k.capital.1).abs();
                    d + vertical*2
                })
                .map(|(_,k)|k.capital);
            for id in self.armies.iter().filter(|a|a.owner==who as u8 && a.troops>0)
                .map(|a|a.id).collect::<Vec<_>>() {
                let needs_order=self.armies.iter().any(|a|a.id==id &&
                    (a.order==Order::Hold || self.tick%180==0));
                if needs_order {
                    if let Some(target)=target {
                        let _=self.issue(who as u8,id,Order::Attack,target);
                    }
                }
            }
        }
    }

    pub fn step(&mut self) {
        if self.winner.is_some(){return;}
        self.tick+=1;
        self.last_report=TurnReport::default();
        if self.tick%15==1 {self.bot_orders();}
        self.advance_armies();
        if self.tick%3==0 {
            self.fight();
            self.claim_land();
        }
        if self.tick%12==0 {
            self.economy();
            self.attrition();
        }
        let active=self.kingdoms.iter().filter(|k|k.alive).count();
        if active<=1 {
            self.winner=self.kingdoms.iter().position(|k|k.alive).map(|i|i as u8);
            return;
        }
        let counts=self.counts();
        let owned:usize=counts.iter().sum();
        if owned>0 {
            if let Some((i,_))=counts.iter().enumerate().find(|(_,n)|**n*100>=owned*78) {
                // Dominance requires retaining a real capital.
                if self.kingdoms[i].alive {self.winner=Some(i as u8);}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_and_commands_are_deterministic() {
        let mut a=World::new(771);
        let mut b=World::new(771);
        let ids=[a.armies[0].id,b.armies[0].id];
        assert!(a.issue(0,ids[0],Order::Attack,(60,11)));
        assert!(b.issue(0,ids[1],Order::Attack,(60,11)));
        for _ in 0..400 {
            a.step();b.step();
            assert_eq!(a,b);
        }
    }

    #[test]
    fn no_claimed_water_and_capitals_have_land() {
        let w=World::new(13);
        for t in &w.tiles {
            if t.terrain==Terrain::Sea {assert_eq!(t.owner,NEUTRAL);}
        }
        for i in 0..FACTIONS {
            let (x,y)=w.kingdoms[i].capital;
            assert_eq!(w.owner_at(x,y),i as u8);
        }
    }

    #[test]
    fn only_existing_army_owner_can_command() {
        let mut w=World::new(3);
        let id=w.armies[0].id;
        assert!(!w.issue(1,id,Order::Attack,(60,11)));
        assert!(w.issue(0,id,Order::Attack,(60,11)));
        assert_eq!(w.armies[0].target,(60,11));
        assert!(w.issue(0,id,Order::Retreat,(60,11)));
        assert_eq!(w.armies[0].target,w.kingdoms[0].capital);
    }

    #[test]
    fn recruitment_uses_real_population_and_food() {
        let mut w=World::new(14);
        let old_pop=w.kingdoms[0].population;
        let old_food=w.kingdoms[0].food;
        let id=w.recruit(0).unwrap();
        assert_eq!(w.kingdoms[0].food,old_food-450);
        assert_eq!(w.kingdoms[0].population,old_pop-180);
        assert_eq!(w.armies.last().unwrap().id,id);
        assert!(!w.issue(1,id,Order::Move,(30,20)));
    }

    #[test]
    fn mountain_and_water_change_travel_possibilities() {
        let w=World::new(15);
        assert!(w.next_step((11,11),(60,11)).is_some());
        for army in &w.armies {
            assert!(w.terrain_at(army.x,army.y).traversable());
        }
    }

    #[test]
    fn winter_has_strictly_lower_food_yield() {
        let mut w=World::new(4);
        let productive: i32=w.tiles.iter().filter(|t|t.owner==0)
            .map(|t|t.terrain.yield_food()).sum();
        assert!(productive>0);
        assert!(productive/5<productive);
        w.tick=320;
        assert!(w.winter());
        w.tick=640;
        assert!(!w.winter());
    }

    #[test]
    fn bots_act_only_through_simulation_ticks() {
        let mut w=World::new(99);
        let before=w.armies[1].target;
        for _ in 0..16 {w.step();}
        assert_ne!(w.armies[1].target,before);
        assert_eq!(w.armies[1].order,Order::Attack);
    }

    #[test]
    fn no_out_of_bounds_units_after_many_ticks() {
        for seed in [1,2,3,17,42] {
            let mut w=World::new(seed);
            for _ in 0..600 {w.step();}
            for a in &w.armies {
                assert!(World::valid(a.x,a.y));
                assert!(w.terrain_at(a.x,a.y).traversable());
            }
            for t in &w.tiles {
                assert!(t.owner==NEUTRAL || t.owner<FACTIONS as u8);
            }
        }
    }
}
