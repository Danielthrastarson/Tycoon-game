//! Genesis V0.4 experimental tactical combat runner.
//! No browser, networking or live-world modification is involved.
//!
//! Example:
//! cargo run --release --bin combat_lab -- --seeds 32 --ablation all \
//!   --output results/combat_v04.csv
//!
//! 16 ablations x 4 scenarios x 4 tactics x 2 side assignments x 32
//! paired seeds = 16,384 deterministic independent match evaluations.

use genesis::frontier_combat::{Battle, CombatRules, Layout, OpponentPolicy, Policy, Scenario};
use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    process,
    time::Instant,
};

const SCREENING_SEED_START: u64 = 1_000;
const HOLDOUT_SEED_START: u64 = 900_000;
const MAX_SEEDS: usize = 2_048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase { Screening, Holdout }
impl Phase {
    fn label(self)->&'static str {
        match self {Self::Screening=>"screening",Self::Holdout=>"holdout"}
    }
}

struct Options {
    seeds: usize,
    masks: Vec<u8>,
    phase: Phase,
    opponent: OpponentPolicy,
    layouts: Vec<Layout>,
    output: String,
}

fn help() -> &'static str {
    "Genesis V0.4 tactical combat laboratory\n\
Usage: combat_lab [--seeds N] [--ablation all|core|full|MASK] [--layout all|classic|ridge|river_ford|split_valley] [--opponent standard|counter|pincer|fortress] [--phase screening|holdout] [--output FILE]\n\
  --seeds      Default 32 for screening, 256 for holdout; max 2048\n\
  --ablation   all = 16 masks; core = 0,7,11,13,14,15; full = mask 15; or 0..15\n\
  --layout     all (default), classic, ridge, river_ford, split_valley\n\
  --opponent   standard (default), counter, pincer, fortress\n\
  --phase      screening uses seeds 1000..; holdout uses previously unused 900000..\n\
  --output     CSV path; defaults to results/combat_v04.csv\n\
Every mask uses the same seeds, positions, scenario geometry, policies and sides.\n\
Winner is determined by physical depot occupation or army collapse, never by strategy name.\n\
A draw includes unresolved timeouts. Draws score 0.5 for exploratory summaries."
}

fn parse_options()->Result<Options,String> {
    let mut phase=Phase::Screening;
    let mut opponent=OpponentPolicy::Standard;
    let mut layouts=Layout::ALL.to_vec();
    let mut explicit_seeds=None::<usize>;
    let mut masks=(0u8..16u8).collect::<Vec<_>>();
    let mut output="results/combat_v04.csv".to_string();
    let mut args=env::args().skip(1);
    while let Some(arg)=args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!("{}",help());
                process::exit(0);
            }
            "--seeds" => {
                let value=args.next().ok_or("--seeds requires a value")?;
                let n=value.parse::<usize>().map_err(|_|"invalid --seeds")?;
                if n==0 || n>MAX_SEEDS {
                    return Err(format!("--seeds must be between 1 and {MAX_SEEDS}"));
                }
                explicit_seeds=Some(n);
            }
            "--opponent" => {
                opponent=match args.next().as_deref() {
                    Some("standard")=>OpponentPolicy::Standard,
                    Some("counter")=>OpponentPolicy::Counter,
                    Some("pincer")=>OpponentPolicy::Pincer,
                    Some("fortress")=>OpponentPolicy::Fortress,
                    _=>return Err("--opponent must be standard, counter, pincer or fortress".into()),
                };
            }
            "--layout" => {
                layouts=match args.next().as_deref() {
                    Some("all")=>Layout::ALL.to_vec(),
                    Some("classic")=>vec![Layout::Classic],
                    Some("ridge")=>vec![Layout::Ridge],
                    Some("river_ford")=>vec![Layout::RiverFord],
                    Some("split_valley")=>vec![Layout::SplitValley],
                    _=>return Err("--layout must be all, classic, ridge, river_ford or split_valley".into()),
                };
            }
            "--phase" => {
                phase=match args.next().as_deref() {
                    Some("screening")=>Phase::Screening,
                    Some("holdout")=>Phase::Holdout,
                    _=>return Err("--phase must be screening or holdout".into()),
                };
            }
            "--ablation" => {
                let value=args.next().ok_or("--ablation requires a value")?;
                masks=match value.as_str() {
                    "all"=>(0..16).collect(),
                    "full"=>vec![15],
                    "core"=>vec![0,7,11,13,14,15],
                    _=>{
                        let n=value.parse::<u8>()
                            .map_err(|_|"invalid --ablation mask")?;
                        if n>15 {return Err("ablation mask must be 0..15".into());}
                        vec![n]
                    }
                };
            }
            "--output" => {
                output=args.next().ok_or("--output requires a file")?;
                if output.is_empty() {return Err("--output requires a file".into());}
            }
            _=>return Err(format!("unknown argument: {arg}")),
        }
    }
    let seeds=explicit_seeds.unwrap_or(match phase {
        Phase::Screening=>32,
        Phase::Holdout=>256,
    });
    Ok(Options{seeds,masks,phase,opponent,layouts,output})
}

#[derive(Clone, Copy, Debug, Default)]
struct Aggregate {
    games: u64,
    // Winner receives two points, draw receives one; divide by 2*games.
    points: u64,
    wins: u64,
    draws: u64,
    remaining: u64,
    turns: u64,
}

fn scenario_num(s:Scenario)->u8 {
    match s { Scenario::MountainPass=>0,Scenario::SupplyCorridor=>1,
        Scenario::OpenField=>2,Scenario::OutnumberedField=>3 }
}
fn policy_num(p:Policy)->u8 {
    match p {
        Policy::DirectRush=>0,Policy::PassDefense=>1,
        Policy::SupplyRaid=>2,Policy::Adaptive=>3,
    }
}
fn seed_at(phase:Phase,i:usize)->u64 {
    match phase {
        Phase::Screening=>SCREENING_SEED_START+i as u64,
        Phase::Holdout=>HOLDOUT_SEED_START+i as u64,
    }
}

fn run(opts:Options)->Result<(),Box<dyn std::error::Error>> {
    let start=Instant::now();
    let destination=Path::new(&opts.output);
    if let Some(parent)=destination.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let file=File::create(destination)?;
    let mut writer=BufWriter::new(file);
    writeln!(writer,"phase,seed_index,seed,scenario,policy,swap,mask,terrain,supply,morale,fatigue,winner,small_win,large_win,draw,rounds,small_remaining,large_remaining,small_combat_lost,large_combat_lost,small_attrition_lost,large_attrition_lost,small_supplied_rounds,large_supplied_rounds,small_isolated_rounds,large_isolated_rounds,small_provisions_used,large_provisions_used,small_retreats,large_retreats,small_engagements,large_engagements,small_objective_rounds,large_objective_rounds,small_depot_lost,large_depot_lost,valid,run_microseconds,victory_reason,pass_hold_rounds,breakthrough_rounds,small_interdiction_rounds,large_interdiction_rounds,small_center_hold_rounds,large_center_hold_rounds,opponent_policy,layout,objective_x,objective_y,water_tiles,map_signature")?;
    let mut totals=BTreeMap::<(u8,u8,u8),Aggregate>::new();
    let mut count=0u64;
    let mut total_micros=0u128;
    for seed_index in 0..opts.seeds {
        let seed=seed_at(opts.phase,seed_index);
        for scenario in Scenario::ALL {
            for policy in Policy::ALL {
                for swap in [false,true] {
                    for &layout in &opts.layouts {
                    for &mask in &opts.masks {
                        let rules=CombatRules::from_mask(mask);
                        let started=Instant::now();
                        let mut battle=Battle::new(seed,scenario,policy,swap,rules)
                            .with_layout(layout)
                            .with_opponent(opts.opponent);
                        battle.play();
                        let us=started.elapsed().as_micros();
                        total_micros+=us;
                        count+=1;
                        let valid=battle.audit();
                        if !valid {
                            return Err(format!(
                                "ledger / troop invariant failed: seed={seed} scenario={:?} policy={:?} mask={mask} swap={swap}",
                                scenario,policy
                            ).into());
                        }
                        let small=battle.remaining(0);
                        let large=battle.remaining(1);
                        let winner=match battle.winner {
                            Some(0)=>"small",Some(1)=>"large",_=>"draw",
                        };
                        let result=if battle.winner==Some(0){2}
                            else if battle.winner.is_none(){1}else{0};
                        let key=(scenario_num(scenario),mask,policy_num(policy));
                        let aggregate=totals.entry(key).or_default();
                        aggregate.games+=1;
                        aggregate.points+=result;
                        aggregate.wins+=(battle.winner==Some(0)) as u64;
                        aggregate.draws+=battle.winner.is_none() as u64;
                        aggregate.remaining+=small as u64;
                        aggregate.turns+=battle.round as u64;
                        let m=battle.stats;
                        let water=battle.map.iter()
                            .filter(|t|**t==genesis::frontier::Terrain::Sea).count();
                        let signature=battle.map.iter().fold(
                            14695981039346656037_u64,|value,t|
                                (value^(*t as u64)).wrapping_mul(1099511628211_u64)
                        );
                        writeln!(writer,
                            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                            opts.phase.label(),seed_index,seed,
                            scenario.label(),policy.label(),swap as u8,mask,
                            rules.terrain as u8,rules.supply as u8,
                            rules.morale as u8,rules.fatigue as u8,winner,
                            (battle.winner==Some(0)) as u8,
                            (battle.winner==Some(1)) as u8,
                            battle.winner.is_none() as u8,
                            battle.round,small,large,
                            m[0].combat_lost,m[1].combat_lost,
                            m[0].attrition_lost,m[1].attrition_lost,
                            m[0].supply_connected_rounds,m[1].supply_connected_rounds,
                            m[0].isolated_rounds,m[1].isolated_rounds,
                            m[0].supply_used,m[1].supply_used,
                            m[0].retreats,m[1].retreats,
                            m[0].engagements,m[1].engagements,
                            m[0].objective_rounds,m[1].objective_rounds,
                            battle.depot_lost[0] as u8,battle.depot_lost[1] as u8,
                            valid as u8,us,
                            battle.victory_reason.map_or("draw",|v|v.label()),
                            battle.pass_hold_rounds,battle.breakthrough_rounds,
                            battle.interdiction_rounds[0],battle.interdiction_rounds[1],
                            battle.center_hold_rounds[0],battle.center_hold_rounds[1],
                            opts.opponent.label(),layout.label(),
                            battle.objective.0,battle.objective.1,water,signature
                        )?;
                    }
                    }
                }
            }
        }
    }
    writer.flush()?;
    eprintln!("GENESIS COMBAT LAB V0.4.3 | phase={} | opponent={} | layouts={} | seeds={} | masks={:?} | runs={} | elapsed={:.2}s",
        opts.phase.label(),opts.opponent.label(),opts.layouts.len(),opts.seeds,opts.masks,count,start.elapsed().as_secs_f64());
    eprintln!("CSV={} | avg simulation time={:.1}us | all invariants PASS",
        opts.output,total_micros as f64/count as f64);
    eprintln!("Exploratory matched-seed policy contrasts (draw scores as 0.5; NOT a confidence interval):");
    for scenario in Scenario::ALL {
        for mask in [15,14,13,11,7,0] {
            if !opts.masks.contains(&mask) {continue;}
            let id=scenario_num(scenario);
            let rush=totals.get(&(id,mask,policy_num(Policy::DirectRush)));
            let adaptive=totals.get(&(id,mask,policy_num(Policy::Adaptive)));
            if let (Some(rush),Some(adaptive))=(rush,adaptive) {
                let score=|a:&Aggregate|a.points as f64/(2*a.games) as f64;
                eprintln!("  {:<18} mask={:>2} adaptive={:>6.1}% rush={:>6.1}% paired-score-difference={:>+6.1} pp draws(adaptive/rush)={}/{}",
                    scenario.label(),mask,score(adaptive)*100.0,score(rush)*100.0,
                    (score(adaptive)-score(rush))*100.0,adaptive.draws,rush.draws);
            }
        }
    }
    eprintln!("Acceptance thresholds remain UNVERIFIED until pre-registered holdout analysis.\n\
    Do not interpret draws as wins or tune game coefficients against holdout seeds.");
    Ok(())
}

fn main() {
    match parse_options() {
        Ok(options)=>{
            if let Err(error)=run(options) {
                eprintln!("Combat laboratory failed: {error}");
                process::exit(1);
            }
        }
        Err(error)=>{
            eprintln!("Argument error: {error}\n{}",help());
            process::exit(2);
        }
    }
}
