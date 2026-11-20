//! Read-only presentation contract shared by independent Genesis experiments.
//!
//! Nothing in this module drives a simulation tick, assigns goals, or mutates
//! ownership. A renderer may rebuild its view from these snapshots, but never
//! use rendered positions as authoritative game state. Coordinates are logical
//! grid cells; a 3D renderer can map (x, y) -> (x, elevation, z).
//!
//! This is deliberately a *projection*, not a merged simulation. Frontier and
//! Society keep their own laws, clocks, scales, and identifiers.

pub const SCENE_SCHEMA_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    Frontier,
    Society,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapTopology {
    /// Out-of-bounds movement is not connected to the opposite edge.
    Bounded,
    /// Opposite edges are adjacent in both logical grid directions.
    Toroidal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Surface {
    Sea,
    Plain,
    Farm,
    Forest,
    Mountain,
    Bare,
    Resource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceState {
    pub stock: u16,
    pub capacity: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneTile {
    pub surface: Surface,
    /// None means unclaimed. A society cell is never pre-assigned territory.
    pub owner: Option<u8>,
    /// Only present for a concrete stock in the authoritative simulation.
    pub resource: Option<ResourceState>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneMap {
    pub width: usize,
    pub height: usize,
    pub topology: MapTopology,
    /// Row-major, tile index = y * width + x.
    pub tiles: Vec<SceneTile>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridPosition {
    pub x: i16,
    pub y: i16,
}

/// Stable within a simulation instance. IDs are namespaced by kind and mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SceneEntityId {
    Person(u32),
    Army(u32),
    Camp(u32),
    Capital(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneEntityDetails {
    Person { energy: u16 },
    Army { troops: u32 },
    Camp { stored: u16 },
    Capital,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneEntity {
    pub id: SceneEntityId,
    pub position: GridPosition,
    /// Current controller of the entity; not necessarily the founding owner.
    pub owner: Option<u8>,
    pub details: SceneEntityDetails,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SceneFrame {
    pub schema_version: u16,
    pub kind: SceneKind,
    pub seed: u64,
    pub tick: u64,
    pub map: SceneMap,
    pub entities: Vec<SceneEntity>,
}

/// A one-way interface: simulation state -> immutable data for presentation.
///
/// Call once per authoritative tick or on demand, not once per GPU frame.
/// A renderer may interpolate between successive entity positions visually.
pub trait SceneSource {
    fn scene_frame(&self) -> SceneFrame;
}

impl SceneSource for crate::frontier::World {
    fn scene_frame(&self) -> SceneFrame {
        use crate::frontier::{Terrain, NEUTRAL, FACTIONS, HEIGHT, WIDTH};
        let tiles = self.tiles.iter().map(|tile| {
            let surface = match tile.terrain {
                Terrain::Sea => Surface::Sea,
                Terrain::Plain => Surface::Plain,
                Terrain::Farm => Surface::Farm,
                Terrain::Forest => Surface::Forest,
                Terrain::Mountain => Surface::Mountain,
            };
            SceneTile {
                surface,
                owner: (tile.owner != NEUTRAL).then_some(tile.owner),
                resource: None, // Frontier has no per-tile stock ledger.
            }
        }).collect();
        let mut entities = Vec::with_capacity(FACTIONS + self.armies.len());
        for (id, kingdom) in self.kingdoms.iter().enumerate() {
            let (x, y) = kingdom.capital;
            // A captured capital remains a physical location. Its controller
            // follows the authoritative tile, not its founding faction.
            let controller = self.tiles[crate::frontier::World::index(x, y)].owner;
            entities.push(SceneEntity {
                id: SceneEntityId::Capital(id as u8),
                position: GridPosition { x, y },
                owner: (controller != NEUTRAL).then_some(controller),
                details: SceneEntityDetails::Capital,
            });
        }
        for army in &self.armies {
            entities.push(SceneEntity {
                id: SceneEntityId::Army(army.id),
                position: GridPosition { x: army.x, y: army.y },
                owner: Some(army.owner),
                details: SceneEntityDetails::Army { troops: army.troops },
            });
        }
        SceneFrame {
            schema_version: SCENE_SCHEMA_VERSION,
            kind: SceneKind::Frontier,
            seed: self.seed,
            tick: self.tick,
            map: SceneMap { width: WIDTH, height: HEIGHT, topology: MapTopology::Bounded, tiles },
            entities,
        }
    }
}

impl SceneSource for crate::society::World {
    fn scene_frame(&self) -> SceneFrame {
        use crate::society::SIDE;
        let tiles = self.tiles.iter().map(|tile| SceneTile {
            surface: if tile.capacity > 0 { Surface::Resource } else { Surface::Bare },
            owner: None, // Social ties are not land ownership.
            resource: (tile.capacity > 0).then_some(ResourceState {
                stock: tile.stock as u16,
                capacity: tile.capacity as u16,
            }),
        }).collect();
        let mut entities = Vec::with_capacity(self.agents.len() + self.camps.len());
        for (id, agent) in self.agents.iter().enumerate() {
            if agent.alive {
                entities.push(SceneEntity {
                    id: SceneEntityId::Person(id as u32),
                    position: GridPosition { x: agent.x, y: agent.y },
                    owner: None,
                    details: SceneEntityDetails::Person { energy: agent.energy },
                });
            }
        }
        for (id, camp) in self.camps.iter().enumerate() {
            entities.push(SceneEntity {
                id: SceneEntityId::Camp(id as u32),
                position: GridPosition { x: camp.x, y: camp.y },
                owner: None, // Camps use a public store, not faction property.
                details: SceneEntityDetails::Camp { stored: camp.supplies },
            });
        }
        SceneFrame {
            schema_version: SCENE_SCHEMA_VERSION,
            kind: SceneKind::Society,
            seed: self.seed,
            tick: self.tick,
            map: SceneMap {
                width: SIDE,
                height: SIDE,
                topology: MapTopology::Toroidal,
                tiles,
            },
            entities,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontier_projection_is_deterministic_and_read_only() {
        let world = crate::frontier::World::new(19);
        let original = world.clone();
        let frame = world.scene_frame();
        assert_eq!(world, original);
        assert_eq!(frame, world.scene_frame());
        assert_eq!(frame.schema_version, 1);
        assert_eq!(frame.kind, SceneKind::Frontier);
        assert_eq!(frame.map.topology, MapTopology::Bounded);
        assert_eq!(frame.map.tiles.len(), crate::frontier::WIDTH * crate::frontier::HEIGHT);
        assert_eq!(frame.entities.iter().filter(|e|
            matches!(e.id, SceneEntityId::Capital(_))).count(), crate::frontier::FACTIONS);
        assert_eq!(frame.entities.iter().filter(|e|
            matches!(e.id, SceneEntityId::Army(_))).count(), world.armies.len());
        for (index, tile) in world.tiles.iter().enumerate() {
            assert_eq!(frame.map.tiles[index].owner,
                (tile.owner != crate::frontier::NEUTRAL).then_some(tile.owner));
        }
    }

    #[test]
    fn society_projection_preserves_resources_and_no_preassigned_ownership() {
        let mut world = crate::society::World::new(19);
        world.agents[0].alive = false;
        world.camps.push(crate::settlements::Camp {
            x: 7, y: 9, builder: 1, built_tick: 1,
            supplies: 23, uses: 0, last_used_tick: 1,
        });
        let original = world.clone();
        let frame = world.scene_frame();
        assert_eq!(world, original);
        assert_eq!(frame, world.scene_frame());
        assert_eq!(frame.kind, SceneKind::Society);
        assert_eq!(frame.map.topology, MapTopology::Toroidal);
        assert_eq!(frame.map.tiles.len(), crate::society::SIDE * crate::society::SIDE);
        assert!(frame.map.tiles.iter().all(|t| t.owner.is_none()));
        assert_eq!(frame.entities.iter().filter(|e|
            matches!(e.id, SceneEntityId::Person(_))).count(), world.alive_count());
        assert!(!frame.entities.iter().any(|e| e.id == SceneEntityId::Person(0)));
        assert!(frame.entities.iter().any(|e| e.id == SceneEntityId::Camp(0)
            && matches!(e.position, GridPosition { x: 7, y: 9 })
            && matches!(e.details, SceneEntityDetails::Camp { stored: 23 })));
        for (index, tile) in world.tiles.iter().enumerate() {
            assert_eq!(frame.map.tiles[index].resource,
                (tile.capacity > 0).then_some(ResourceState {
                    stock: tile.stock as u16, capacity: tile.capacity as u16,
                }));
        }
    }

    #[test]
    fn frames_follow_authoritative_ticks_without_driving_them() {
        let mut world = crate::frontier::World::new(11);
        let initial = world.scene_frame();
        world.tick += 1;
        let later = world.scene_frame();
        assert_eq!(initial.tick + 1, later.tick);
        assert_eq!(initial.map, later.map);
    }
}
