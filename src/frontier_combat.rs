//! Genesis Frontier V0.4 experimental combat rules.
//!
//! This is a SEPARATE, deterministic headless tactical model. It deliberately
//! does not modify the published Frontier World, game rules, or renderer.
//! Coefficients are hypotheses; only validated mechanics should be promoted.
//!
//! Two teams each control a main force and a smaller detachment. They use
//! actual movement, finite provisions, positional combat and observable state.
//! All 16 ablation masks reuse the SAME seeded battlefield and starting teams.

use crate::frontier::{Order, Terrain};
use std::collections::VecDeque;

pub const FIELD_WIDTH: i16 = 15;
pub const FIELD_HEIGHT: i16 = 9;
pub const MAX_ROUNDS: u16 = 80;
const CELLS: usize = FIELD_WIDTH as usize * FIELD_HEIGHT as usize;
// Enough for an entire 80-round battle when lines remain connected.
// Thus losing supply measures interdiction, not a universal stockpile timer.
const START_RESERVE: u16 = 400;
const START_PROVISIONS: u8 = 6;
const PASS: (i16, i16) = (7, 4);

/// Four reproducible cartographic archetypes. Physical barriers remain the
/// same under every experimental combat-factor mask.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layout {
    Classic, Ridge, RiverFord, SplitValley,
}
impl Layout {
    pub const ALL:[Self;4]=[
        Self::Classic,Self::Ridge,Self::RiverFord,Self::SplitValley
    ];
    pub fn label(self)->&'static str {
        match self {
            Self::Classic=>"classic",
            Self::Ridge=>"ridge",
            Self::RiverFord=>"river_ford",
            Self::SplitValley=>"split_valley",
        }
    }
}

/// Explicitly observable mission objectives, not bonuses to a chosen AI.
pub const MOUNTAIN_DEFEND_ROUNDS: u16 = 24;
pub const BREAKTHROUGH_ROUNDS: u8 = 3;
pub const INTERDICTION_ROUNDS: u8 = 5;
pub const OPEN_CONTROL_ROUNDS: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatRules {
    pub terrain: bool,
    pub supply: bool,
    pub morale: bool,
    pub fatigue: bool,
}

impl CombatRules {
    pub const FULL: Self = Self {
        terrain: true, supply: true, morale: true, fatigue: true,
    };

    /// Bit order: terrain (1), supply (2), morale (4), fatigue (8).
    pub fn from_mask(mask: u8) -> Self {
        Self {
            terrain: mask & 1 != 0,
            supply: mask & 2 != 0,
            morale: mask & 4 != 0,
            fatigue: mask & 8 != 0,
        }
    }

    pub fn mask(self) -> u8 {
        (self.terrain as u8)
            | ((self.supply as u8) << 1)
            | ((self.morale as u8) << 2)
            | ((self.fatigue as u8) << 3)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario { MountainPass, SupplyCorridor, OpenField, OutnumberedField }
impl Scenario {
    pub const ALL: [Self; 4] = [
        Self::MountainPass, Self::SupplyCorridor,
        Self::OpenField, Self::OutnumberedField,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::MountainPass => "mountain_pass",
            Self::SupplyCorridor => "supply_corridor",
            Self::OpenField => "open_field",
            Self::OutnumberedField => "outnumbered_field",
        }
    }
}

/// Scenarios declare physical victory conditions independently of the
/// strategy or ablation mask. A time limit without an objective is a draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VictoryReason {
    Annihilation,
    DepotCapture,
    PassHeld,
    Breakthrough,
    SupplyInterdicted,
    CenterSecured,
}
impl VictoryReason {
    pub fn label(self)->&'static str {
        match self {
            Self::Annihilation=>"annihilation",
            Self::DepotCapture=>"depot_capture",
            Self::PassHeld=>"pass_held",
            Self::Breakthrough=>"breakthrough",
            Self::SupplyInterdicted=>"supply_interdicted",
            Self::CenterSecured=>"center_secured",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy { DirectRush, PassDefense, SupplyRaid, Adaptive }
impl Policy {
    pub const ALL: [Self; 4] = [
        Self::DirectRush, Self::PassDefense, Self::SupplyRaid, Self::Adaptive,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::DirectRush => "direct_rush",
            Self::PassDefense => "pass_defense",
            Self::SupplyRaid => "supply_raid",
            Self::Adaptive => "adaptive",
        }
    }
}

/// The standard opponent is the V0.4.2 reference baseline. Counter is
/// a tougher, responsive adversary evaluated as a separate stress test.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpponentPolicy { Standard, Counter, Pincer, Fortress }
impl OpponentPolicy {
    pub const ALL:[Self;4]=[
        Self::Standard,Self::Counter,Self::Pincer,Self::Fortress
    ];
    pub fn label(self)->&'static str {
        match self {
            Self::Standard=>"standard",Self::Counter=>"counter",
            Self::Pincer=>"pincer",Self::Fortress=>"fortress",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role { Main, Scout }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TacticalState {
    pub morale: u8,        // 0..=100
    pub fatigue: u8,       // 0..=100
    pub provisions: u8,    // 0..=6 consumed during rounds
    pub entrenchment: u8,  // 0..=3, HOLD only
}

impl Default for TacticalState {
    fn default() -> Self {
        Self { morale: 100, fatigue: 0,
            provisions: START_PROVISIONS, entrenchment: 0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unit {
    pub side: u8,
    pub role: Role,
    pub pos: (i16, i16),
    pub troops: u32,
    pub state: TacticalState,
    pub order: Order,
    pub destination: (i16, i16),
    pub routing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct TeamStats {
    pub combat_lost: u32,
    pub attrition_lost: u32,
    pub supply_connected_rounds: u32,
    pub isolated_rounds: u32,
    pub supply_used: u32,
    pub retreats: u32,
    pub engagements: u32,
    pub objective_rounds: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Battle {
    pub scenario: Scenario,
    pub layout: Layout,
    /// Actual objective coordinate, varied by seeded map geometry.
    pub objective: (i16,i16),
    pub policy: Policy,
    pub opponent: OpponentPolicy,
    pub rules: CombatRules,
    pub seed: u64,
    /// Which physical side the smaller/policy-controlled team occupies.
    pub swap: bool,
    pub round: u16,
    pub map: [Terrain; CELLS],
    pub units: [Unit; 4],
    pub depots: [(i16, i16); 2],
    /// One depot per team; captured means it no longer supplies that team.
    pub depot_lost: [bool; 2],
    pub reserve: [u16; 2],
    pub stats: [TeamStats; 2],
    pub objective_owner: Option<u8>,
    pub winner: Option<u8>,
    pub victory_reason: Option<VictoryReason>,
    pub ended: bool,
    /// Three turns physically occupying an undefended enemy depot wins.
    depot_occupation: [u8; 2],
    /// Physical objective progress. All counts are reset on contest/loss.
    pub pass_hold_rounds: u16,
    pub breakthrough_rounds: u8,
    pub interdiction_rounds: [u8; 2],
    pub center_hold_rounds: [u8; 2],
    /// Tracks exact conservation of provision tokens, even after combat deaths.
    /// Supplies may be destroyed with a destroyed army, but never materialize.
    pub destroyed_provisions: [u32; 2],
}

fn hash(seed: u64, salt: u64) -> u32 {
    let mut x=seed ^ salt.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= x >> 30;
    x=x.wrapping_mul(0xBF58476D1CE4E5B9);
    x ^= x >> 27;
    x=x.wrapping_mul(0x94D049BB133111EB);
    (x ^ (x >> 31)) as u32
}

fn distance(a:(i16,i16),b:(i16,i16))->i16 {
    (a.0-b.0).abs()+(a.1-b.1).abs()
}

fn clamp_pos(p:(i16,i16))->(i16,i16) {
    (p.0.clamp(0,FIELD_WIDTH-1),p.1.clamp(0,FIELD_HEIGHT-1))
}

fn index(p:(i16,i16))->usize {
    p.1 as usize*FIELD_WIDTH as usize+p.0 as usize
}

impl Battle {
    pub fn new(seed:u64,scenario:Scenario,policy:Policy,swap:bool,
        rules:CombatRules)->Self {
        let mut map=[Terrain::Plain;CELLS];
        if scenario==Scenario::MountainPass {
            // Narrow pass is represented as a terrain combat modifier only.
            // All cells remain traversable in every ablation configuration.
            for y in 2..=6 {
                map[index((7,y))]=if y==4 {Terrain::Mountain}
                    else {Terrain::Forest};
            }
        }
        if scenario==Scenario::SupplyCorridor {
            for x in 4..=10 {
                for y in [1,2,6,7] {
                    if hash(seed,(x*17+y) as u64)%3==0 {
                        map[index((x,y))]=Terrain::Forest;
                    }
                }
            }
        }
        let depots=if swap {[(13,4),(1,4)]} else {[(1,4),(13,4)]};
        let left=if swap {11} else {3};
        let right=if swap {3} else {11};
        // Scenario A is explicitly a defense: the 400-troop army begins
        // occupying the pass and must decide whether to hold or abandon it.
        // Scenario B starts the smaller army nearer the contested road.
        let main_small=match scenario {
            Scenario::MountainPass=>PASS.0,
            Scenario::SupplyCorridor=>if swap {8} else {6},
            Scenario::OpenField | Scenario::OutnumberedField=>if swap {9} else {5},
        };
        let main_large=if swap {4} else {10};
        let offset=(hash(seed,19)%3) as i16-1;
        let small_y=if scenario==Scenario::MountainPass {4}else{4+offset};
        let small_troops=if scenario==Scenario::OpenField {500} else {400};
        let large_troops=if scenario==Scenario::OpenField {500} else {650};
        let split_small=small_troops*3/4;
        let split_large=large_troops*4/5;
        let make_unit=|side:u8,role:Role,pos:(i16,i16),troops:u32|{
            Unit { side,role,pos, troops,state:TacticalState::default(),
                order:Order::Hold,destination:pos,routing:false }
        };
        Self {
            scenario,layout:Layout::Classic,objective:PASS,
            policy,opponent:OpponentPolicy::Standard,rules,
            seed,swap,round:0,map,
            units:[
                make_unit(0,Role::Main,(main_small,small_y),split_small),
                make_unit(0,Role::Scout,(left,1),small_troops-split_small),
                make_unit(1,Role::Main,(main_large,4-offset),split_large),
                make_unit(1,Role::Scout,(right,3),large_troops-split_large),
            ],
            depots,depot_lost:[false,false],
            reserve:[START_RESERVE;2],
            stats:[TeamStats::default();2],
            objective_owner:None,
            winner:None,victory_reason:None,ended:false,
            depot_occupation:[0,0],
            pass_hold_rounds:0,
            breakthrough_rounds:0,
            interdiction_rounds:[0,0],
            center_hold_rounds:[0,0],
            destroyed_provisions:[0,0],
        }
    }

    pub fn with_opponent(mut self, opponent:OpponentPolicy)->Self {
        self.opponent=opponent;
        self
    }

    /// Rebuild only the experimental geography and initial positions.
    /// This method must be used before advancing any combat rounds.
    pub fn with_layout(mut self, layout:Layout)->Self {
        self.layout=layout;
        if layout==Layout::Classic {return self;}
        let cy=2+(hash(self.seed,0x4f02+layout as u64)%5) as i16;
        self.objective=(7,cy);
        self.depots=if self.swap {[(13,cy),(1,cy)]} else {[(1,cy),(13,cy)]};
        let mut map=[Terrain::Plain;CELLS];
        for y in 0..FIELD_HEIGHT {
            for x in 2..FIELD_WIDTH-2 {
                let n=hash(self.seed,((x*23+y*43) as u64)+layout as u64*719);
                if n%9==0 {map[index((x,y))]=Terrain::Forest;}
                else if n%23==0 {map[index((x,y))]=Terrain::Mountain;}
            }
        }
        match layout {
            Layout::Classic=>unreachable!(),
            Layout::Ridge=>{
                // Thick traversable ridgeline with widely varying defense.
                for y in 0..FIELD_HEIGHT {
                    map[index((7,y))]=Terrain::Mountain;
                    if (y-cy).abs()<=2 {
                        map[index((6,y))]=Terrain::Forest;
                        map[index((8,y))]=Terrain::Forest;
                    }
                }
            }
            Layout::RiverFord=>{
                // Water barrier with two genuine crossings, including the
                // objective's ford. The flanking ford varies with seed.
                let second=if cy<=4 {7}else{1};
                for y in 0..FIELD_HEIGHT {
                    if y!=cy && y!=second {map[index((7,y))]=Terrain::Sea;}
                    else {map[index((7,y))]=Terrain::Mountain;}
                }
            }
            Layout::SplitValley=>{
                // Double barrier, staggered east/west alternate passes.
                let second=if cy<=4 {7}else{1};
                for y in 0..FIELD_HEIGHT {
                    if y!=cy && y!=second {
                        map[index((6,y))]=Terrain::Sea;
                    }
                    if y!=cy && y!=(8-second) {
                        map[index((8,y))]=Terrain::Sea;
                    }
                }
                map[index((7,cy))]=Terrain::Mountain;
            }
        }
        // No mission or initial army may be placed inside water.
        map[index(self.objective)]=if self.scenario==Scenario::MountainPass {
            Terrain::Mountain
        } else {Terrain::Plain};
        for d in self.depots {map[index(d)]=Terrain::Plain;}
        self.units[0].pos=if self.scenario==Scenario::MountainPass {
            self.objective
        } else {(if self.swap {9}else{5},cy)};
        self.units[1].pos=(if self.swap {11}else{3},1);
        self.units[2].pos=(if self.swap {4}else{10},cy);
        self.units[3].pos=(if self.swap {3}else{11},7);
        for u in &mut self.units {
            map[index(u.pos)]=Terrain::Plain;
            u.destination=u.pos;
            u.order=Order::Hold;
        }
        if self.scenario==Scenario::MountainPass {
            map[index(self.objective)]=Terrain::Mountain;
        }
        // Seed-dependent road texture and ridgeline are shared across all
        // policies, side assignments and ablations for one map/seed pairing.
        self.map=map;
        self
    }

    pub fn terrain_at(&self,p:(i16,i16))->Terrain {
        self.map[index(clamp_pos(p))]
    }

    pub fn remaining(&self,team:usize)->u32 {
        self.units.iter().filter(|u|u.side as usize==team)
            .map(|u|u.troops).sum()
    }

    pub fn initial_troops(&self,team:usize)->u32 {
        if self.scenario==Scenario::OpenField {500}
        else if team==0 {400} else {650}
    }

    fn breakthrough_target(&self)->(i16,i16) {
        if self.swap {(10,self.objective.1)} else {(4,self.objective.1)}
    }

    fn corridor_target(&self)->(i16,i16) {
        let depot=self.depots[1];
        if depot.0>self.objective.0 {
            (10,self.objective.1)
        } else {(4,self.objective.1)}
    }

    pub fn supplies_balanced(&self,team:usize)->bool {
        // Sum of issued stocks + carried stocks + consumed + destroyed
        // equals initial finite stocks. Disabled supply conserves all tokens.
        let carried:u32=self.units.iter().filter(|u|u.side as usize==team)
            .map(|u|u.state.provisions as u32).sum();
        self.reserve[team] as u32 + carried +
            self.stats[team].supply_used + self.destroyed_provisions[team]
            == START_RESERVE as u32 + START_PROVISIONS as u32*2
    }

    /// The depot-to-unit corridor can be interdicted by a physical enemy.
    /// No global "cut supply" command or automatic teleport of provisions.
    pub fn connected(&self,idx:usize)->bool {
        let u=self.units[idx];
        let side=u.side as usize;
        if u.troops==0 || self.depot_lost[side] {return false;}
        let depot=self.depots[side];
        if distance(u.pos,depot)>14 {return false;}
        for enemy in self.units.iter().filter(|e|e.side!=u.side && e.troops>0) {
            if distance(enemy.pos,depot)<=1 {return false;}
            let behind=if depot.0<u.pos.0 {
                enemy.pos.0>=depot.0 && enemy.pos.0<u.pos.0
            } else {
                enemy.pos.0<=depot.0 && enemy.pos.0>u.pos.0
            };
            if behind && (enemy.pos.1-self.objective.1).abs()<=1 {
                return false;
            }
        }
        true
    }

    /// Breadth-first movement through the actual grid. Opposing armies and
    /// sea are impassable; if an enemy holds the target, route to the closest
    /// accessible adjacent square. Geographic combat modifiers may be
    /// ablated without making sea traversable.
    fn route_step(
        map:&[Terrain;CELLS], occupancy:&[Unit;4],
        from:(i16,i16), target:(i16,i16), side:u8
    )->Option<(i16,i16)> {
        if from==target {return None;}
        let mut parents=[usize::MAX;CELLS];
        let mut length=[u16::MAX;CELLS];
        let mut queue=VecDeque::new();
        let start=index(from);
        parents[start]=start;length[start]=0;
        queue.push_back(from);
        let mut best=start;
        let mut closest=distance(from,target);
        let offsets=[(1,0),(0,-1),(-1,0),(0,1)];
        while let Some((x,y))=queue.pop_front() {
            let current=index((x,y));
            let d=distance((x,y),target);
            if d<closest || (d==closest && length[current]<length[best]) {
                closest=d;best=current;
            }
            if (x,y)==target {break;}
            for (dx,dy) in offsets {
                let pos=(x+dx,y+dy);
                if pos.0<0 || pos.0>=FIELD_WIDTH ||
                    pos.1<0 || pos.1>=FIELD_HEIGHT {continue;}
                let idx=index(pos);
                if parents[idx]!=usize::MAX || map[idx]==Terrain::Sea {
                    continue;
                }
                if occupancy.iter().any(|enemy|enemy.side!=side &&
                    enemy.troops>0 && enemy.pos==pos) {continue;}
                parents[idx]=current;
                length[idx]=length[current]+1;
                queue.push_back(pos);
            }
        }
        if best==start {return None;}
        let mut cursor=best;
        while parents[cursor]!=start {
            cursor=parents[cursor];
            if cursor==usize::MAX {return None;}
        }
        Some(((cursor%FIELD_WIDTH as usize) as i16,
            (cursor/FIELD_WIDTH as usize) as i16))
    }

    fn assign(&mut self,idx:usize,order:Order,destination:(i16,i16)) {
        if self.units[idx].troops==0 {return;}
        let order=if self.units[idx].routing {
            Order::Retreat
        } else {order};
        let target=if order==Order::Retreat {
            self.depots[self.units[idx].side as usize]
        } else {clamp_pos(destination)};
        let u=&mut self.units[idx];
        u.order=order;
        u.destination=if order==Order::Hold {u.pos} else {target};
    }

    fn scout_raid_destination(&self,idx:usize)->(Order,(i16,i16)) {
        let u=self.units[idx];
        let cut=self.corridor_target();
        // Flank along the northern rim, THEN approach the road from behind
        // the opponent's main body. A raid is not a magical command:
        // the scout must physically reach the vulnerable road square.
        if u.pos.0!=cut.0 {
            (Order::Move,(cut.0,1))
        } else if u.pos.1!=cut.1 {
            (Order::Attack,cut)
        } else {
            (Order::Hold,cut)
        }
    }

    fn plan(&mut self) {
        // All tactics read only the current public map and current unit state.
        // They do not use future RNG draws or the ablation configuration.
        let big_main=self.units[2];
        let big_scout=self.units[3];
        let small_main=self.units[0];
        let small_scout=self.units[1];
        let small_depot=self.depots[0];
        let big_depot=self.depots[1];

        let style=match self.policy {
            Policy::Adaptive => if self.scenario==Scenario::MountainPass {
                Policy::PassDefense
            } else {
                // Road and central-point objectives reward staying on an
                // objective while the scout threatens enemy depth.
                Policy::SupplyRaid
            },
            other=>other,
        };
        match style {
            Policy::DirectRush => {
                self.assign(0,Order::Attack,big_main.pos);
                self.assign(1,Order::Attack,big_scout.pos);
            }
            Policy::PassDefense => {
                if small_main.pos==self.objective {
                    self.assign(0,Order::Hold,self.objective);
                } else {
                    self.assign(0,Order::Attack,self.objective);
                }
                // Defense-in-depth: protect the *actual* breakthrough tile.
                // A rear guard two tiles away cannot contest occupation.
                let defensive=if self.scenario==Scenario::MountainPass {
                    self.breakthrough_target()
                } else {
                    let x=if self.swap {9} else {5};
                    (x,3)
                };
                if small_scout.pos==defensive {
                    self.assign(1,Order::Hold,defensive);
                } else {
                    self.assign(1,Order::Move,defensive);
                }
            }
            Policy::SupplyRaid => {
                if small_main.pos==self.objective {
                    self.assign(0,Order::Hold,self.objective);
                } else {
                    self.assign(0,Order::Attack,self.objective);
                }
                let (order,target)=self.scout_raid_destination(1);
                self.assign(1,order,target);
            }
            Policy::Adaptive => unreachable!(),
        }
        if self.policy==Policy::Adaptive {
            // Actual local tactical state can override the simple scenario
            // preference. This is not an omniscient win condition.
            if small_main.state.morale<42 || small_main.state.fatigue>88 {
                self.assign(0,Order::Retreat,small_depot);
            } else if self.scenario==Scenario::SupplyCorridor &&
                small_main.pos!=self.objective && !self.physical_interdiction(0) &&
                (big_main.state.provisions==0 || big_main.troops<small_main.troops/2) {
                // Never abandon a held road interdiction merely because
                // the isolated enemy is running low on provisions.
                self.assign(0,Order::Attack,big_main.pos);
            }
        }

        // Four independent, deterministic enemy command policies. None
        // sees future state, rule toggles or inaccessible resource bonuses.
        let counter=self.opponent==OpponentPolicy::Counter;
        let pincer=self.opponent==OpponentPolicy::Pincer;
        let fortress=self.opponent==OpponentPolicy::Fortress;
        let flank_after=if pincer {1} else if counter {2}
            else if fortress {6} else {8};
        let beyond_pass=if self.swap {big_main.pos.0>=8}
            else {big_main.pos.0<=6};
        if self.scenario==Scenario::MountainPass && beyond_pass {
            self.assign(2,Order::Attack,self.breakthrough_target());
        } else if self.scenario==Scenario::MountainPass &&
            self.pass_hold_rounds>=flank_after &&
            ((self.swap && big_main.pos.0<=7) ||
             (!self.swap && big_main.pos.0>=7)) {
            let lane=if counter || pincer {7} else {1};
            let flank=if self.swap {(9,lane)} else {(5,lane)};
            self.assign(2,Order::Attack,flank);
        } else if distance(big_main.pos,small_main.pos)<=1 && small_main.troops>0 {
            self.assign(2,Order::Attack,small_main.pos);
        } else if self.scenario==Scenario::MountainPass {
            self.assign(2,Order::Attack,self.breakthrough_target());
        } else if self.scenario==Scenario::SupplyCorridor {
            let raider=self.units[1];
            let response_range=if fortress {7} else if counter {4} else {1};
            if raider.troops>0 &&
                distance(raider.pos,self.corridor_target())<=response_range {
                self.assign(2,Order::Attack,raider.pos);
            } else if pincer {
                self.assign(2,Order::Attack,self.objective);
            } else {
                self.assign(2,Order::Attack,small_main.pos);
            }
        } else if counter || pincer || fortress {
            // Contest the actual seeded center rather than a fixed tile.
            if fortress && big_main.pos==self.objective {
                self.assign(2,Order::Hold,self.objective);
            } else {
                self.assign(2,Order::Attack,self.objective);
            }
        } else {
            self.assign(2,Order::Attack,small_main.pos);
        }
        let guard_radius=if fortress {8} else if counter {7} else {3};
        let intruder=[small_main,small_scout].into_iter()
            .filter(|u|u.troops>0 && distance(u.pos,big_depot)<=guard_radius)
            .min_by_key(|u|distance(u.pos,big_depot));
        if pincer && self.scenario==Scenario::MountainPass &&
            self.pass_hold_rounds>0 {
            // Second army tries a different approach: either a real
            // breakthrough from the north or a force-on-force clash.
            let lane=1;
            let target=if self.swap {(10,lane)} else {(4,lane)};
            self.assign(3,Order::Attack,target);
        } else if pincer && self.scenario==Scenario::SupplyCorridor {
            self.assign(3,Order::Attack,small_depot);
        } else if pincer &&
            matches!(self.scenario,Scenario::OpenField|Scenario::OutnumberedField) {
            self.assign(3,Order::Attack,self.objective);
        } else if self.scenario==Scenario::MountainPass && counter &&
            self.pass_hold_rounds>=2 && small_scout.troops>0 {
            self.assign(3,Order::Attack,small_scout.pos);
        } else if fortress &&
            matches!(self.scenario,Scenario::OpenField|Scenario::OutnumberedField) {
            if big_scout.pos==self.objective {
                self.assign(3,Order::Hold,self.objective);
            } else {
                self.assign(3,Order::Attack,self.objective);
            }
        } else if let Some(u)=intruder {
            self.assign(3,Order::Attack,u.pos);
        } else if counter &&
            matches!(self.scenario,Scenario::OpenField|Scenario::OutnumberedField) {
            self.assign(3,Order::Attack,self.objective);
        } else if distance(big_scout.pos,big_depot)>1 {
            self.assign(3,Order::Move,big_depot);
        } else {
            self.assign(3,Order::Hold,big_depot);
        }
    }

    fn movement(&mut self)->[bool;4] {
        let mut moved=[false;4];
        // All units read the same pre-movement occupancy. We also track
        // destinations claimed earlier this round to avoid endpoint stacking
        // and move-update advantages (there are only four units).
        let before=self.units;
        let map=self.map;
        let mut claimed:Vec<(i16,i16)>=Vec::new();
        for i in 0..4 {
            let u=&mut self.units[i];
            if u.troops==0 {continue;}
            if u.order==Order::Hold {
                u.state.entrenchment=u.state.entrenchment.saturating_add(1).min(3);
            } else {
                u.state.entrenchment=0;
                let steps=if u.order==Order::Retreat {2} else {1};
                for _ in 0..steps {
                    let next=Self::route_step(&map,&before,u.pos,u.destination,u.side);
                    let Some(next)=next else {break;};
                    if claimed.contains(&next) {break;}
                    if before.iter().any(|enemy|enemy.side!=u.side &&
                        enemy.troops>0 && enemy.pos==next) {break;}
                    u.pos=next;
                    moved[i]=true;
                }
            }
            claimed.push(u.pos);
            if self.rules.fatigue {
                u.state.fatigue=if moved[i] {
                    u.state.fatigue.saturating_add(4).min(100)
                } else if u.order==Order::Hold {
                    u.state.fatigue.saturating_sub(8)
                } else {u.state.fatigue.saturating_sub(2)};
            }
        }
        moved
    }

    fn provision_step(&mut self) {
        if !self.rules.supply {return;}
        let connected:[bool;4]=std::array::from_fn(|i|self.connected(i));
        for i in 0..4 {
            if self.units[i].troops==0 {continue;}
            let side=self.units[i].side as usize;
            if connected[i] {
                self.stats[side].supply_connected_rounds+=1;
            } else {
                self.stats[side].isolated_rounds+=1;
            }
            let carried=&mut self.units[i].state.provisions;
            if *carried>0 {
                *carried-=1;
                self.stats[side].supply_used+=1;
            }
            if connected[i] && *carried<START_PROVISIONS && self.reserve[side]>0 {
                // Real conserved transfer: no food appears from nowhere.
                self.reserve[side]-=1;
                *carried+=1;
            }
        }
    }

    fn attack_power(&self,attacker:Unit,defender:Unit)->u32 {
        if attacker.troops==0 {return 0;}
        let mut power=attacker.troops;
        if self.rules.terrain && self.terrain_at(defender.pos)==Terrain::Mountain
            && defender.order==Order::Hold {
            power=power.min(180); // restricted attack frontage
        }
        let posture:u32=match attacker.order {
            Order::Attack=>100,Order::Hold=>85,Order::Move=>55,Order::Retreat=>20,
        };
        power=power*posture/100;
        if self.rules.supply && attacker.state.provisions==0 {
            power=power*70/100;
        }
        if self.rules.morale {
            power=power*(60+attacker.state.morale as u32*40/100)/100;
        }
        if self.rules.fatigue {
            power=power*(100-attacker.state.fatigue as u32/2)/100;
        }
        power
    }

    fn incoming_damage(&self,attacker:Unit,defender:Unit)->u32 {
        let power=self.attack_power(attacker,defender);
        if power==0 {return 0;}
        let mut damage=(power/26).max(1);
        if defender.order==Order::Hold {
            let multiplier=100u32.saturating_sub(defender.state.entrenchment as u32*10);
            damage=(damage*multiplier/100).max(1);
        }
        if self.rules.terrain {
            damage=match self.terrain_at(defender.pos) {
                Terrain::Mountain=>(damage*70/100).max(1),
                Terrain::Forest=>(damage*90/100).max(1),
                _=>damage,
            };
        }
        damage
    }

    fn fighting(&mut self)->[u32;4] {
        let mut damage=[0u32;4];
        let old=self.units;
        for i in 0..2 {
            for j in 2..4 {
                let a=old[i];
                let b=old[j];
                if a.troops==0 || b.troops==0 ||
                    distance(a.pos,b.pos)>1 {continue;}
                // MOVE is transit, not conquest; it may still be attacked
                // and defend itself when the other side initiates a clash.
                if a.order!=Order::Attack && b.order!=Order::Attack &&
                    a.order!=Order::Hold && b.order!=Order::Hold {continue;}
                damage[i]=damage[i].saturating_add(self.incoming_damage(b,a));
                damage[j]=damage[j].saturating_add(self.incoming_damage(a,b));
                self.stats[0].engagements+=1;
                self.stats[1].engagements+=1;
            }
        }
        for i in 0..4 {
            let old_troops=old[i].troops;
            let actual=damage[i].min(old_troops);
            if actual==0 {continue;}
            let side=old[i].side as usize;
            self.units[i].troops-=actual;
            self.stats[side].combat_lost+=actual;
            if self.rules.morale {
                let shock=(actual*90/old_troops.max(1)).max(2) as u8;
                self.units[i].state.morale=
                    self.units[i].state.morale.saturating_sub(shock);
            }
            if self.rules.fatigue {
                self.units[i].state.fatigue=
                    self.units[i].state.fatigue.saturating_add(6).min(100);
            }
            if self.units[i].troops==0 {
                // Supplies carried by casualties are destroyed, not restored.
                self.destroyed_provisions[side]+=
                    self.units[i].state.provisions as u32;
                self.units[i].state.provisions=0;
            }
        }
        damage
    }

    fn after_combat(&mut self) {
        for i in 0..4 {
            if self.units[i].troops==0 {continue;}
            let side=self.units[i].side as usize;
            if self.rules.supply && self.units[i].state.provisions==0 {
                let lost=if self.round%3==0 {
                    (self.units[i].troops/50).max(1)
                } else {0};
                let lost=lost.min(self.units[i].troops);
                self.units[i].troops-=lost;
                self.stats[side].attrition_lost+=lost;
                if lost>0 && self.rules.morale {
                    self.units[i].state.morale=
                        self.units[i].state.morale.saturating_sub(4);
                }
                if self.units[i].troops==0 {
                    self.destroyed_provisions[side]+=
                        self.units[i].state.provisions as u32;
                    self.units[i].state.provisions=0;
                    continue;
                }
            }
            if self.rules.morale {
                if self.units[i].state.morale<=25 && !self.units[i].routing {
                    self.units[i].routing=true;
                    self.stats[side].retreats+=1;
                }
                if self.units[i].routing {
                    if distance(self.units[i].pos,self.depots[side])<=1 &&
                        self.units[i].state.morale>=45 {
                        self.units[i].routing=false;
                    } else {
                        self.units[i].order=Order::Retreat;
                        self.units[i].destination=self.depots[side];
                    }
                }
                let nearby_enemy=self.units.iter().any(|enemy|
                    enemy.side!=self.units[i].side && enemy.troops>0
                    && distance(enemy.pos,self.units[i].pos)<=2);
                if !nearby_enemy && self.units[i].state.provisions>0 {
                    self.units[i].state.morale=
                        self.units[i].state.morale.saturating_add(2).min(100);
                }
            }
        }
    }

    /// A supply raid must be a living unit physically on the narrow
    /// depot-to-army corridor. Merely commanding RAID or being out of range
    /// cannot satisfy the five-round interdiction objective.
    fn physical_interdiction(&self,side:usize)->bool {
        let defender=1-side;
        let enemy=&self.units[defender*2];
        if enemy.troops==0 {return false;}
        let depot=self.depots[defender];
        self.units.iter().any(|u|u.side as usize==side &&
            u.troops>=15 && (u.pos.1-self.objective.1).abs()<=1 &&
            (if depot.0<enemy.pos.0 {
                u.pos.0>depot.0 && u.pos.0<enemy.pos.0
            } else {
                u.pos.0<depot.0 && u.pos.0>enemy.pos.0
            }) &&
            // A present enemy screen actually contests the cut. The
            // raider cannot merely stand under fire to collect a free win.
            !self.units.iter().any(|guard|guard.side!=u.side &&
                guard.troops>0 && distance(guard.pos,u.pos)<=1))
    }

    fn resolve_objectives(&mut self) {
        // Mutual depot capture is available in EVERY scenario to both teams.
        // Occupation must persist for three rounds without a nearby defender.
        for side in 0..2usize {
            let attacker=side as u8;
            let defender=(1-side) as u8;
            let enemy_depot=self.depots[1-side];
            let occupying=self.units.iter().any(|u|u.side==attacker &&
                u.troops>0 && u.pos==enemy_depot && u.order==Order::Attack);
            let defended=self.units.iter().any(|u|u.side==defender &&
                u.troops>0 && distance(u.pos,enemy_depot)<=1);
            if occupying && !defended {
                self.depot_occupation[side]=self.depot_occupation[side]
                    .saturating_add(1);
                if self.depot_occupation[side]>=3 {
                    self.depot_lost[1-side]=true;
                }
            } else {self.depot_occupation[side]=0;}
        }
        let at_pass:[bool;2]=std::array::from_fn(|side|
            self.units.iter().any(|u|u.side as usize==side &&
                u.troops>0 && distance(u.pos,self.objective)<=1));
        self.objective_owner=match at_pass {
            [true,false]=>Some(0),[false,true]=>Some(1),_=>None,
        };
        if let Some(side)=self.objective_owner {
            self.stats[side as usize].objective_rounds+=1;
        }

        // Time-based military objectives require continuous PHYSICAL control
        // and have the same terrain, clocks and criteria in every ablation.
        let mut mission=[false;2];
        let mut mission_reason=[None;2];
        match self.scenario {
            Scenario::MountainPass=>{
                // Team 0 explicitly defends the pass for a fixed siege period.
                // Team 1 tries to cross and occupy a real tile beyond it.
                let defending=self.units.iter().any(|u|u.side==0 &&
                    u.troops>=40 && u.pos==self.objective && u.order==Order::Hold);
                let occupying_pass=self.units.iter().any(|u|u.side==1 &&
                    u.troops>0 && u.pos==self.objective);
                if defending && !occupying_pass {
                    self.pass_hold_rounds=self.pass_hold_rounds.saturating_add(1);
                } else {self.pass_hold_rounds=0;}

                let breakthrough=self.breakthrough_target();
                let attacker=self.units.iter().any(|u|u.side==1 &&
                    u.troops>=40 && u.pos==breakthrough && u.order==Order::Attack);
                let defended=self.units.iter().any(|u|u.side==0 &&
                    u.troops>0 && distance(u.pos,breakthrough)<=1);
                if attacker && !defended {
                    self.breakthrough_rounds=self.breakthrough_rounds
                        .saturating_add(1);
                } else {self.breakthrough_rounds=0;}
                if self.pass_hold_rounds>=MOUNTAIN_DEFEND_ROUNDS {
                    mission[0]=true;
                    mission_reason[0]=Some(VictoryReason::PassHeld);
                }
                if self.breakthrough_rounds>=BREAKTHROUGH_ROUNDS {
                    mission[1]=true;
                    mission_reason[1]=Some(VictoryReason::Breakthrough);
                }
            }
            Scenario::SupplyCorridor=>{
                // No rule-mask check: the mission is physical road denial,
                // independent of whether starvation penalties are enabled.
                for side in 0..2 {
                    let enemy_main=if side==0 {2}else{0};
                    let blocked=self.physical_interdiction(side) &&
                        !self.connected(enemy_main);
                    self.interdiction_rounds[side]=if blocked {
                        self.interdiction_rounds[side].saturating_add(1)
                    } else {0};
                    if self.interdiction_rounds[side]>=INTERDICTION_ROUNDS {
                        mission[side]=true;
                        mission_reason[side]=Some(VictoryReason::SupplyInterdicted);
                    }
                }
            }
            Scenario::OpenField | Scenario::OutnumberedField=>{
                // Both sides race to exclusively occupy the center. Existing
                // front lines are contested; attackers cannot win by merely
                // walking through an adjacent enemy.
                for side in 0..2 {
                    self.center_hold_rounds[side]=if self.objective_owner==
                        Some(side as u8) {
                        self.center_hold_rounds[side].saturating_add(1)
                    } else {0};
                    if self.center_hold_rounds[side]>=OPEN_CONTROL_ROUNDS {
                        mission[side]=true;
                        mission_reason[side]=Some(VictoryReason::CenterSecured);
                    }
                }
            }
        }

        let remaining=[self.remaining(0),self.remaining(1)];
        let defeated=[remaining[0]<40,remaining[1]<40];
        if defeated[0] && defeated[1] {
            self.winner=None;
            self.victory_reason=None;
            self.ended=true;
        } else if defeated[0] || defeated[1] {
            let winner=if defeated[0] {1} else {0};
            self.winner=Some(winner);
            self.victory_reason=Some(VictoryReason::Annihilation);
            self.ended=true;
        } else {
            for side in 0..2 {
                if self.depot_occupation[side]>=3 {
                    mission[side]=true;
                    mission_reason[side]=Some(VictoryReason::DepotCapture);
                }
            }
            match mission {
                [true,false]=>{
                    self.winner=Some(0);
                    self.victory_reason=mission_reason[0];
                    self.ended=true;
                }
                [false,true]=>{
                    self.winner=Some(1);
                    self.victory_reason=mission_reason[1];
                    self.ended=true;
                }
                [true,true]=>{
                    // Mutually achieved objectives are a genuine draw:
                    // no hidden priority for the first team in the array.
                    self.winner=None;
                    self.victory_reason=None;
                    self.ended=true;
                }
                _=>{}
            }
        }
        if self.round>=MAX_ROUNDS {
            self.ended=true;
        }
    }

    pub fn step(&mut self) {
        if self.ended {return;}
        self.round+=1;
        self.plan();
        self.movement();
        self.provision_step();
        self.fighting();
        self.after_combat();
        self.resolve_objectives();
    }

    pub fn play(&mut self) {
        while !self.ended {self.step();}
    }

    pub fn audit(&self)->bool {
        for side in 0..2 {
            let lost=self.stats[side].combat_lost+self.stats[side].attrition_lost;
            if self.remaining(side)+lost!=self.initial_troops(side) {
                return false;
            }
            if !self.supplies_balanced(side) {return false;}
        }
        self.units.iter().all(|u|u.pos==clamp_pos(u.pos)
            && u.state.morale<=100 && u.state.fatigue<=100
            && u.state.provisions<=START_PROVISIONS
            && u.state.entrenchment<=3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_conditions_fit_the_tactical_hypotheses() {
        for swapped in [false,true] {
            for seed in [1,7,1003] {
                let mountain=Battle::new(seed,Scenario::MountainPass,
                    Policy::PassDefense,swapped,CombatRules::FULL);
                assert_eq!(mountain.units[0].pos,PASS);
                assert_eq!(mountain.terrain_at(PASS),Terrain::Mountain);
                assert_eq!(mountain.initial_troops(0),400);
                assert_eq!(mountain.initial_troops(1),650);
                let open=Battle::new(seed,Scenario::OpenField,
                    Policy::Adaptive,swapped,CombatRules::FULL);
                assert_eq!(open.initial_troops(0),500);
                assert_eq!(open.initial_troops(1),500);
            }
        }
        assert!(START_RESERVE>=MAX_ROUNDS*2,
            "two continuously supplied units must not automatically
             exhaust their depot before the match timer");
    }

    #[test]
    fn sixteen_masks_are_distinct_and_reversible() {
        for mask in 0..16 {
            let rules=CombatRules::from_mask(mask);
            assert_eq!(rules.mask(),mask);
        }
    }

    #[test]
    fn seeds_and_sides_match_across_ablations() {
        for scenario in Scenario::ALL {
            for seed in [0,1,7,1003] {
                for swapped in [false,true] {
                    let a=Battle::new(seed,scenario,Policy::Adaptive,swapped,
                        CombatRules::from_mask(0));
                    let b=Battle::new(seed,scenario,Policy::Adaptive,swapped,
                        CombatRules::FULL);
                    assert_eq!(a.map,b.map);
                    assert_eq!(a.units,b.units);
                    assert_eq!(a.depots,b.depots);
                    assert_eq!(a.reserve,b.reserve);
                }
            }
        }
    }

    #[test]
    fn exact_replay_and_resource_ledger() {
        for scenario in Scenario::ALL {
            for policy in Policy::ALL {
                for mask in [0,1,2,4,8,15] {
                    let mut a=Battle::new(3341,scenario,policy,false,
                        CombatRules::from_mask(mask));
                    let mut b=a.clone();
                    for _ in 0..MAX_ROUNDS {
                        a.step();b.step();
                        assert_eq!(a,b);
                        assert!(a.audit(), "scenario={scenario:?} policy={policy:?} mask={mask} round={}",a.round);
                    }
                }
            }
        }
    }

    #[test]
    fn terrain_ablation_changes_only_combat_effects() {
        let mut a=Battle::new(5,Scenario::MountainPass,
            Policy::PassDefense,false,CombatRules::FULL);
        let mut b=a.clone();
        b.rules.terrain=false;
        // Both opponents on exactly the same physical positions; only
        // defending terrain multipliers may change the damage.
        a.units[0].pos=PASS;b.units[0].pos=PASS;
        a.units[0].order=Order::Hold;b.units[0].order=Order::Hold;
        a.units[0].state.entrenchment=3;b.units[0].state.entrenchment=3;
        a.units[2].pos=(8,4);b.units[2].pos=(8,4);
        a.units[2].order=Order::Attack;b.units[2].order=Order::Attack;
        let terrain=a.incoming_damage(a.units[2],a.units[0]);
        let no_terrain=b.incoming_damage(b.units[2],b.units[0]);
        assert!(terrain<no_terrain);
        assert_eq!(a.map,b.map);
    }

    #[test]
    fn disabling_supply_removes_penalty_without_creating_food() {
        let mut a=Battle::new(21,Scenario::SupplyCorridor,
            Policy::DirectRush,false,CombatRules::FULL);
        let mut b=a.clone();
        b.rules.supply=false;
        a.units[0].state.provisions=0;
        b.units[0].state.provisions=0;
        // Lost carried provision tokens must be counted explicitly.
        a.destroyed_provisions[0]+=6;b.destroyed_provisions[0]+=6;
        let target=a.units[2];
        assert!(a.attack_power(a.units[0],target)<
            b.attack_power(b.units[0],target));
        assert!(a.audit() && b.audit());
    }

    #[test]
    fn fatigue_and_morale_have_separate_effects() {
        let mut a=Battle::new(99,Scenario::OpenField,
            Policy::DirectRush,false,CombatRules::FULL);
        a.units[0].state.morale=35;
        a.units[0].state.fatigue=90;
        let defender=a.units[2];
        let full=a.attack_power(a.units[0],defender);
        let mut no_fatigue=a.clone();no_fatigue.rules.fatigue=false;
        let mut no_morale=a.clone();no_morale.rules.morale=false;
        assert!(full<no_fatigue.attack_power(a.units[0],defender));
        assert!(full<no_morale.attack_power(a.units[0],defender));
    }

    #[test]
    fn occupation_must_be_physical_and_uncontested() {
        let mut a=Battle::new(1024,Scenario::SupplyCorridor,
            Policy::SupplyRaid,false,CombatRules::FULL);
        let enemy=a.depots[1];
        a.units[1].pos=enemy;a.units[1].order=Order::Attack;
        a.units[3].pos=enemy; // guard prevents capture
        a.resolve_objectives();
        assert!(!a.depot_lost[1]);
        a.units[3].pos=(10,1);
        a.resolve_objectives();
        assert!(!a.depot_lost[1]);
        a.resolve_objectives();
        assert!(!a.depot_lost[1]);
        a.resolve_objectives();
        assert!(a.depot_lost[1]);
        assert_eq!(a.winner,Some(0));
        assert_eq!(a.victory_reason,Some(VictoryReason::DepotCapture));
    }

    #[test]
    fn mountain_victory_requires_actual_held_pass_for_full_duration() {
        let mut battle=Battle::new(12,Scenario::MountainPass,
            Policy::PassDefense,false,CombatRules::FULL);
        assert_eq!(battle.units[0].pos,PASS);
        for _ in 0..(MOUNTAIN_DEFEND_ROUNDS-1) {
            battle.resolve_objectives();
            assert!(battle.winner.is_none());
        }
        // The attacker physically occupies the tile, so the streak resets.
        battle.units[2].pos=PASS;
        battle.resolve_objectives();
        assert_eq!(battle.pass_hold_rounds,0);
        assert!(battle.winner.is_none());
        battle.units[2].pos=(10,4);
        for _ in 0..MOUNTAIN_DEFEND_ROUNDS {
            battle.resolve_objectives();
        }
        assert_eq!(battle.winner,Some(0));
        assert_eq!(battle.victory_reason,Some(VictoryReason::PassHeld));
    }

    #[test]
    fn breakthrough_requires_uncontested_occupation_beyond_pass() {
        for swap in [false,true] {
            let mut battle=Battle::new(41,Scenario::MountainPass,
                Policy::DirectRush,swap,CombatRules::FULL);
            let goal=battle.breakthrough_target();
            battle.units[2].pos=goal;
            battle.units[2].order=Order::Attack;
            // Explicit defender at the goal blocks the breakthrough.
            battle.units[0].pos=goal;
            battle.resolve_objectives();
            assert_eq!(battle.breakthrough_rounds,0);
            battle.units[0].pos=battle.depots[0];
            for _ in 0..(BREAKTHROUGH_ROUNDS-1) {
                battle.resolve_objectives();
                assert!(battle.winner.is_none());
            }
            battle.resolve_objectives();
            assert_eq!(battle.winner,Some(1));
            assert_eq!(battle.victory_reason,Some(VictoryReason::Breakthrough));
        }
    }

    #[test]
    fn road_denial_requires_living_unit_behind_enemy_for_five_rounds() {
        for mask in [0,15] {
            let mut battle=Battle::new(21,Scenario::SupplyCorridor,
                Policy::SupplyRaid,false,CombatRules::from_mask(mask));
            // Merely commanding ATTACK cannot disrupt logistics.
            battle.units[1].order=Order::Attack;
            battle.resolve_objectives();
            assert_eq!(battle.interdiction_rounds[0],0);
            battle.units[2].pos=PASS; // enemy main has advanced eastward
            battle.units[1].pos=battle.corridor_target();
            assert!(!battle.connected(2));
            for _ in 0..(INTERDICTION_ROUNDS-1) {
                battle.resolve_objectives();
                assert!(battle.winner.is_none());
            }
            battle.resolve_objectives();
            assert_eq!(battle.winner,Some(0));
            assert_eq!(battle.victory_reason,
                Some(VictoryReason::SupplyInterdicted));
        }
    }

    #[test]
    fn enemy_screen_contests_road_interdiction_instead_of_awarding_free_win() {
        let mut battle=Battle::new(42,Scenario::SupplyCorridor,
            Policy::SupplyRaid,false,CombatRules::FULL);
        battle.units[2].pos=PASS; // enemy main advancing from depot
        battle.units[1].pos=battle.corridor_target();
        battle.units[3].pos=(11,4); // physical counter-screen
        for _ in 0..INTERDICTION_ROUNDS+1 {
            battle.resolve_objectives();
        }
        assert_eq!(battle.interdiction_rounds[0],0);
        assert!(battle.winner.is_none());
        battle.units[3].pos=(13,4); // screen withdraws
        for _ in 0..INTERDICTION_ROUNDS {
            battle.resolve_objectives();
        }
        assert_eq!(battle.winner,Some(0));
        assert_eq!(battle.victory_reason,
            Some(VictoryReason::SupplyInterdicted));
    }

    #[test]
    fn center_control_requires_exclusive_physical_presence() {
        for scenario in [Scenario::OpenField,Scenario::OutnumberedField] {
            let mut battle=Battle::new(56,scenario,
                Policy::Adaptive,false,CombatRules::FULL);
            battle.units[0].pos=PASS;
            battle.units[2].pos=PASS; // contested
            battle.resolve_objectives();
            assert_eq!(battle.center_hold_rounds,[0,0]);
            battle.units[2].pos=(10,4);
            battle.units[3].pos=(11,3);
            for _ in 0..OPEN_CONTROL_ROUNDS {
                battle.resolve_objectives();
            }
            assert_eq!(battle.winner,Some(0));
            assert_eq!(battle.victory_reason,
                Some(VictoryReason::CenterSecured));
        }
    }

    #[test]
    fn a_front_line_blocks_enemy_phasing_even_during_simultaneous_movement() {
        let mut battle=Battle::new(86,Scenario::OpenField,
            Policy::DirectRush,false,CombatRules::FULL);
        battle.units[0].pos=(7,4);
        battle.units[0].destination=(9,4);
        battle.units[0].order=Order::Attack;
        battle.units[2].pos=(8,4);
        battle.units[2].destination=(7,4);
        battle.units[2].order=Order::Attack;
        battle.movement();
        assert_eq!(battle.units[0].pos,(7,4));
        assert_eq!(battle.units[2].pos,(8,4));
    }

    #[test]
    fn defense_orders_guard_real_breakthrough_objective() {
        for swap in [false,true] {
            let mut battle=Battle::new(1000,Scenario::MountainPass,
                Policy::PassDefense,swap,CombatRules::FULL);
            battle.plan();
            assert_eq!(battle.units[1].destination,
                battle.breakthrough_target());
            assert_eq!(battle.units[0].order,Order::Hold);
        }
    }

    #[test]
    fn adaptive_keeps_pass_held_when_supply_is_being_interdicted() {
        let mut battle=Battle::new(1000,Scenario::SupplyCorridor,
            Policy::Adaptive,false,CombatRules::FULL);
        battle.units[0].pos=PASS;
        battle.units[2].pos=(5,4);
        battle.units[2].state.provisions=0;
        battle.destroyed_provisions[1]+=6;
        battle.plan();
        assert_eq!(battle.units[0].order,Order::Hold);
        assert!(battle.audit());
    }

    #[test]
    fn counter_opponent_keeps_identical_starting_world_and_replays_exactly() {
        for scenario in Scenario::ALL {
            for swap in [false,true] {
                let standard=Battle::new(123,scenario,
                    Policy::Adaptive,swap,CombatRules::FULL);
                let counter=standard.clone().with_opponent(OpponentPolicy::Counter);
                assert_eq!(standard.units,counter.units);
                assert_eq!(standard.map,counter.map);
                assert_eq!(standard.reserve,counter.reserve);
                assert_eq!(standard.depots,counter.depots);
                assert_ne!(standard.opponent,counter.opponent);
                let mut a=counter.clone();
                let mut b=counter;
                for _ in 0..MAX_ROUNDS {
                    a.step();b.step();
                    assert_eq!(a,b);
                    assert!(a.audit());
                }
            }
        }
    }

    #[test]
    fn numerical_control_is_distinct_from_equal_force_control() {
        let equal=Battle::new(77,Scenario::OpenField,
            Policy::Adaptive,false,CombatRules::FULL);
        let unequal=Battle::new(77,Scenario::OutnumberedField,
            Policy::Adaptive,false,CombatRules::FULL);
        assert_eq!((equal.initial_troops(0),equal.initial_troops(1)),
            (500,500));
        assert_eq!((unequal.initial_troops(0),unequal.initial_troops(1)),
            (400,650));
    }

    #[test]
    fn disabled_mechanics_do_not_mutate_the_original_game_world() {
        let original=crate::frontier::World::new(7733);
        let before=original.clone();
        for mask in 0..16 {
            let mut experiment=Battle::new(7733,Scenario::MountainPass,
                Policy::Adaptive,false,CombatRules::from_mask(mask));
            experiment.play();
            assert!(experiment.audit());
        }
        assert_eq!(original,before);
    }
}
