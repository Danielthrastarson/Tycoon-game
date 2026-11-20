//! Export authoritative Frontier ticks for the Godot 3D visual benchmark.
//! This is a read-only *recording*. Godot does not drive the Rust simulation;
//! the future live adapter must replace this recording with a command channel.
//! Usage: cargo run --release --bin scene_export -- godot/scene_data/frontier_demo.json

use genesis::frontier::{Order, World, NORTH_CROSSING};
use genesis::scene::{SceneEntityDetails, SceneSource, SceneTile, Surface};
use std::fmt::Write as _;

fn surface_id(tile: SceneTile) -> u8 {
    match tile.surface {
        Surface::Sea => 0,
        Surface::Plain => 1,
        Surface::Farm => 2,
        Surface::Forest => 3,
        Surface::Mountain => 4,
        Surface::Bare => 5,
        Surface::Resource => 6,
    }
}

fn export(seed: u64, ticks: usize) -> String {
    let mut world = World::new(seed);
    let player_id = world.armies.iter().find(|a| a.owner == 0).map(|a| a.id);
    let rival_id = world.armies.iter().find(|a| a.owner == 1).map(|a| a.id);
    if let Some(id) = player_id {
        assert!(world.issue(0, id, Order::Attack, NORTH_CROSSING));
    }
    if let Some(id) = rival_id {
        assert!(world.issue(1, id, Order::Attack, NORTH_CROSSING));
    }

    let first = world.scene_frame();
    let mut out = String::with_capacity(75_000);
    write!(
        out,
        "{{\"source\":\"rust-frontier\",\"schema_version\":{},\"seed\":{},\"width\":{},\"height\":{},\"surface\":[",
        first.schema_version, seed, first.map.width, first.map.height
    ).unwrap();

    for (i, tile) in first.map.tiles.iter().enumerate() {
        if i != 0 { out.push(','); }
        write!(out, "{}", surface_id(*tile)).unwrap();
    }
    out.push_str("],\"owners\":[");
    for (i, tile) in first.map.tiles.iter().enumerate() {
        if i != 0 { out.push(','); }
        match tile.owner {
            Some(owner) => write!(out, "{owner}").unwrap(),
            None => out.push_str("-1"),
        }
    }
    out.push_str("],\"frames\":[");

    for step in 0..=ticks {
        if step != 0 { out.push(','); }
        let frame = world.scene_frame();
        write!(out, "{{\"tick\":{},\"armies\":[", frame.tick).unwrap();
        let mut first_army = true;
        for army in &frame.entities {
            if let GenesisArmy::Present(id, owner, x, y, troops) = extract_army(army) {
                // Two actual, front-line armies rather than thousands of fake agents.
                if Some(id) != player_id && Some(id) != rival_id { continue; }
                if troops == 0 { continue; }
                if !first_army { out.push(','); }
                first_army = false;
                write!(out, "[{id},{owner},{x},{y},{troops}]").unwrap();
            }
        }
        out.push_str("]}");
        if step < ticks { world.step(); }
    }
    out.push_str("]}");
    out
}

enum GenesisArmy { Present(u32, u8, i16, i16, u32), Other }

fn extract_army(entity: &genesis::scene::SceneEntity) -> GenesisArmy {
    if let (genesis::scene::SceneEntityId::Army(id),
        SceneEntityDetails::Army { troops }) = (entity.id, entity.details)
    {
        if let Some(owner) = entity.owner {
            return GenesisArmy::Present(id, owner, entity.position.x, entity.position.y, troops);
        }
    }
    GenesisArmy::Other
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1)
        .unwrap_or_else(|| "godot/scene_data/frontier_demo.json".into());
    let seed = 77001;
    let frames = export(seed, 420);
    if let Some(parent) = std::path::Path::new(&path).parent() {
        if !parent.as_os_str().is_empty() { std::fs::create_dir_all(parent)?; }
    }
    std::fs::write(&path, frames.as_bytes())?;
    println!("Exported 421 authoritative Frontier snapshots to {path}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exporter_deterministic_and_has_two_real_armies() {
        let a = export(77001, 4);
        let b = export(77001, 4);
        assert_eq!(a, b);
        assert!(a.contains("\"source\":\"rust-frontier\""));
        assert!(a.contains("\"tick\":4"));
        assert!(a.contains("\"armies\":[["));
        assert!(!a.contains("NaN"));
    }
}
