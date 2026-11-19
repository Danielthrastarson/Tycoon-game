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

pub const FIELD_WIDTH: i16 = 15;
pub const FIELD_HEIGHT: i16 = 9;
pub const MAX_ROUNDS: u16 = 80;
const CELLS: usize = FIELD_WIDTH as usize * FIELD_HEIGHT as usize;
// Enough for an entire 80-round battle when lines remain connected.
// Thus losing supply measures interdiction, not a universal stockpile timer.
const START_RESERVE: u16 = 400;
const START_PROVISIONS: u8 = 6;
const PASS: (i16, i16) = (7, 4);

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
pub enum Scenario { MountainPass, SupplyCorridor, OpenField }
impl Scenario {
    pub const ALL: [Self; 3] = [
        Self::MountainPass, Self::SupplyCorridor, Self::OpenField,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::MountainPass => "mountain_pass",
            Self::SupplyCorridor => "supply_corridor",
            Self::OpenField => "open_field",
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
    pub policy: Policy,
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
    pub ended: bool,
    /// Three turns physically occupying an undefended enemy depot wins.
    depot_occupation: [u8; 2],
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
            Scenario::OpenField=>if swap {9} else {5},
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
            scenario,policy,rules,seed,swap,round:0,map,
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
            winner:None,ended:false,
            depot_occupation:[0,0],
            destroyed_provisions:[0,0],
        }
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
            if behind && (enemy.pos.1-4).abs()<=1 {
                return false;
            }
        }
        true
    }

    fn move_one(pos:(i16,i16),target:(i16,i16))->(i16,i16) {
        if pos.1!=target.1 {
            (pos.0,pos.1+if target.1>pos.1 {1}else{-1})
        } else if pos.0!=target.0 {
            (pos.0+if target.0>pos.0 {1}else{-1},pos.1)
        } else {pos}
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
        let depot=self.depots[1];
        // A real flank: first travel on the northern edge, then turn onto
        // the enemy supply depot. May be intercepted by a garrison.
        if u.pos.1!=1 {
            (Order::Move,(u.pos.0,1))
        } else if u.pos.0!=depot.0 {
            (Order::Move,(depot.0,1))
        } else {
            (Order::Attack,depot)
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
            } else if self.scenario==Scenario::SupplyCorridor {
                Policy::SupplyRaid
            } else {Policy::DirectRush},
            other=>other,
        };
        match style {
            Policy::DirectRush => {
                self.assign(0,Order::Attack,big_main.pos);
                self.assign(1,Order::Attack,big_scout.pos);
            }
            Policy::PassDefense => {
                if small_main.pos==PASS {
                    self.assign(0,Order::Hold,PASS);
                } else {
                    self.assign(0,Order::Attack,PASS);
                }
                let x=if self.swap {9} else {5};
                let defensive=(x,3);
                if small_scout.pos==defensive {
                    self.assign(1,Order::Hold,defensive);
                } else {
                    self.assign(1,Order::Move,defensive);
                }
            }
            Policy::SupplyRaid => {
                if small_main.pos==PASS {
                    self.assign(0,Order::Hold,PASS);
                } else {
                    self.assign(0,Order::Attack,PASS);
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
            } else if self.scenario!=Scenario::OpenField &&
                (big_main.state.provisions==0 || big_main.troops<small_main.troops/2) {
                self.assign(0,Order::Attack,big_main.pos);
            }
        }

        // Fixed baseline opponent: move its main body toward the central
        // fight; its smaller detachment defends the depot from nearby raiders.
        // It obeys exactly the same supply, combat, movement and morale rules.
        if distance(big_main.pos,small_main.pos)<=1 {
            self.assign(2,Order::Attack,small_main.pos);
        } else if self.scenario==Scenario::MountainPass {
            self.assign(2,Order::Attack,PASS);
        } else {
            self.assign(2,Order::Attack,small_main.pos);
        }
        let intruder=[small_main,small_scout].into_iter()
            .filter(|u|u.troops>0 && distance(u.pos,big_depot)<=3)
            .min_by_key(|u|distance(u.pos,big_depot));
        if let Some(u)=intruder {
            self.assign(3,Order::Attack,u.pos);
        } else if distance(big_scout.pos,big_depot)>1 {
            self.assign(3,Order::Move,big_depot);
        } else {
            self.assign(3,Order::Hold,big_depot);
        }
    }

    fn movement(&mut self)->[bool;4] {
        let mut moved=[false;4];
        for (i,u) in self.units.iter_mut().enumerate() {
            if u.troops==0 {continue;}
            if u.order==Order::Hold {
                u.state.entrenchment=u.state.entrenchment.saturating_add(1).min(3);
            } else {
                u.state.entrenchment=0;
                let steps=if u.order==Order::Retreat {2} else {1};
                for _ in 0..steps {
                    let next=Self::move_one(u.pos,u.destination);
                    if next==u.pos {break;}
                    u.pos=clamp_pos(next);
                    moved[i]=true;
                }
            }
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

    fn resolve_objectives(&mut self) {
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
                if self.depot_occupation[side]>=2 {
                    self.depot_lost[1-side]=true;
                }
            } else {self.depot_occupation[side]=0;}
        }
        let at_pass:[bool;2]=std::array::from_fn(|side|
            self.units.iter().any(|u|u.side as usize==side &&
                u.troops>0 && distance(u.pos,PASS)<=1));
        self.objective_owner=match at_pass {
            [true,false]=>Some(0),[false,true]=>Some(1),_=>None,
        };
        if let Some(side)=self.objective_owner {
            self.stats[side as usize].objective_rounds+=1;
        }

        let remaining=[self.remaining(0),self.remaining(1)];
        if remaining[0]<40 && remaining[1]>=40 {
            self.winner=Some(1);
        } else if remaining[1]<40 && remaining[0]>=40 {
            self.winner=Some(0);
        } else if remaining[0]<40 && remaining[1]<40 {
            self.winner=None;self.ended=true;
        } else if self.depot_occupation[0]>=3 {
            self.winner=Some(0);
        } else if self.depot_occupation[1]>=3 {
            self.winner=Some(1);
        }
        if self.winner.is_some() || self.round>=MAX_ROUNDS {
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
        assert!(a.depot_lost[1]);
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
