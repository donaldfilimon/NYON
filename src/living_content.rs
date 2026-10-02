//! Versioned, validated Living starting content. These are ordinary genesis
//! manifests, with no privileged runtime scripting or injected story receipts.
use nyon_workshop_core::living::*;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LivingContent {
    ValeConfluence,
    Commerce,
    FrontierFriction,
    InterruptedCorridor,
    FirstExpansion,
    Blank,
    Seeded(u64),
}
impl LivingContent {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ValeConfluence => "Vale Confluence",
            Self::Commerce => "Commerce",
            Self::FrontierFriction => "Frontier Friction",
            Self::InterruptedCorridor => "Interrupted Corridor",
            Self::FirstExpansion => "First Expansion",
            Self::Blank => "Blank Galaxy",
            Self::Seeded(_) => "Seeded Galaxy",
        }
    }
    pub fn open(self) -> Result<LivingGalaxyAuthorityV2, String> {
        if self == Self::FirstExpansion {
            return builtin_first_expansion_v2();
        }
        let seed = match self {
            Self::Seeded(seed) => seed,
            Self::Blank => 0,
            Self::Commerce => 0x4e59_4f4e_434f_4d4d,
            Self::FrontierFriction => 0x4e59_4f4e_4652_4f4e,
            Self::InterruptedCorridor => 0x4e59_4f4e_494f_4e53,
            _ => 0x4e59_4f4e_5641_4c45,
        };
        let generator = match self {
            Self::Blank => "blank_galaxy",
            Self::Seeded(_) => "seeded_galaxy",
            Self::Commerce => "commerce",
            Self::FrontierFriction => "frontier_friction",
            Self::InterruptedCorridor => "interrupted_corridor",
            _ => "vale_confluence",
        };
        let state = match self {
            Self::Blank => LivingGalaxyStateV2::default(),
            Self::Seeded(_) => generated(seed, 12, 24, 4, false)?,
            _ => {
                let mut state = generated(seed, 4, 8, 3, true)?;
                match self {
                    Self::Commerce | Self::InterruptedCorridor => {
                        configure_commerce(&mut state, seed, self == Self::InterruptedCorridor)?
                    }
                    Self::FrontierFriction => configure_frontier(&mut state)?,
                    Self::ValeConfluence => {
                        let world = world_named(&state, "Aster Vale")?.id;
                        let mut worlds = state.worlds.as_slice().to_vec();
                        worlds
                            .iter_mut()
                            .find(|row| row.id == world)
                            .ok_or("starter world absent")?
                            .inventory
                            .energy = 0;
                        state.worlds = rows(worlds)?;
                        state.facilities = rows(
                            state
                                .facilities
                                .as_slice()
                                .iter()
                                .filter(|row| {
                                    row.world != world || row.definition.as_str() != "solar_array"
                                })
                                .cloned()
                                .collect(),
                        )?;
                    }
                    _ => {}
                }
                state
            }
        };
        authority(state, seed, generator)
    }
}

fn authority(
    state: LivingGalaxyStateV2,
    seed: u64,
    generator: &str,
) -> Result<LivingGalaxyAuthorityV2, String> {
    let catalog = living_core_pack_v2().map_err(|error| error.to_string())?;
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .map_err(|error| error.to_string())?;
    let mut seed_bytes = [0; 32];
    seed_bytes[24..].copy_from_slice(&seed.to_be_bytes());
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        seed_bytes,
        LivingGenesisGeneratorV2 {
            generator_id: slug(generator)?,
            generator_version: 1,
        },
        genesis,
    )
    .map_err(|error| error.to_string())
}
fn id(seed: u64, ordinal: u64) -> LivingEntityIdV2 {
    let mut hash = Sha256::new();
    hash.update(b"NYON-CLIENT-GENESIS-ENTITY-V1\0");
    hash.update(seed.to_be_bytes());
    hash.update(ordinal.to_be_bytes());
    let digest = hash.finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    LivingEntityIdV2(bytes)
}
fn slug(value: &str) -> Result<LivingSlugV2, String> {
    LivingSlugV2::new(value).map_err(|error| error.to_string())
}
fn name(value: &str) -> Result<LivingNameV2, String> {
    LivingNameV2::new(value).map_err(|error| error.to_string())
}
fn rows<T: LivingKeyedV2>(mut rows: Vec<T>) -> Result<LivingSortedVecV2<T>, String> {
    rows.sort_by_key(LivingKeyedV2::living_key);
    LivingSortedVecV2::new(rows).map_err(|error| error.to_string())
}

fn generated(
    seed: u64,
    system_count: usize,
    world_count: usize,
    civ_count: usize,
    curated: bool,
) -> Result<LivingGalaxyStateV2, String> {
    let mut state = LivingGalaxyStateV2::default();
    let mut next = 1;
    let mut allocate = || {
        let value = id(seed, next);
        next += 1;
        value
    };
    let mut systems = Vec::new();
    let mut stars = Vec::new();
    let mut worlds = Vec::new();
    let mut civs = Vec::new();
    let mut colonies = Vec::new();
    let mut facilities = Vec::new();
    let mut deposits = Vec::new();
    let mut lanes = Vec::new();
    let mut fleets = Vec::new();
    for index in 0..system_count {
        let system = allocate();
        let system_name = if curated {
            ["Vale", "Nacre", "Crucible", "Confluence"][index].to_owned()
        } else {
            format!("System {}", index + 1)
        };
        systems.push(LivingSystemV2 {
            id: system,
            name: name(&system_name)?,
        });
        stars.push(LivingStarV2 {
            id: allocate(),
            system,
            archetype: slug("main_sequence")?,
            name: name(&format!("{system_name} Star"))?,
        });
    }
    for index in 0..civ_count {
        let (display, policy) = if curated {
            [
                ("Vale Cooperative", "trader"),
                ("Nacre Cartographers", "expansionist"),
                ("Crucible Directorate", "guardian"),
            ][index]
        } else {
            (
                "Civilization",
                ["trader", "expansionist", "guardian", "industrialist"][index % 4],
            )
        };
        civs.push(LivingCivilizationV2 {
            id: allocate(),
            name: name(&if curated {
                display.to_owned()
            } else {
                format!("{display} {}", index + 1)
            })?,
            policy: LivingPolicyV2(slug(policy)?),
            status: LivingCivilizationStatusV2::Active,
        });
    }
    for index in 0..world_count {
        let world = allocate();
        let system = systems[index * system_count / world_count].id;
        let owner = if index < civ_count * 2 {
            Some(civs[index / 2].id)
        } else {
            None
        };
        let display = if curated {
            [
                "Aster Vale",
                "Vale Annex",
                "Nacre",
                "Nacre Reach",
                "Crucible",
                "Crucible Forge",
                "Far Lantern",
                "Quiet Reach",
            ][index]
                .to_owned()
        } else {
            format!("World {}", index + 1)
        };
        worlds.push(LivingWorldV2 {
            id: world,
            system,
            archetype: slug(if owner.is_some() {
                "temperate"
            } else {
                "barren"
            })?,
            name: name(&display)?,
            owner,
            inventory: if owner.is_some() {
                LivingInventoryV2 {
                    energy: 100,
                    ore: 100,
                    alloy: 120,
                }
            } else {
                LivingInventoryV2::default()
            },
        });
        if let Some(owner) = owner {
            colonies.push(LivingColonyV2 {
                world,
                owner,
                hub: LivingHubV2 {
                    energy_next_due: LivingTickV2(10),
                    ore_next_due: LivingTickV2(100),
                    fallback_next_due: LivingTickV2(100),
                    blocked_reasons: LivingSortedVecV2::default(),
                },
            });
            deposits.push(LivingDepositV2 {
                id: allocate(),
                world,
                resource: LivingResourceV2::Ore,
                remaining_units: 20000,
            });
            for (definition, due) in [
                ("solar_array", Some(10)),
                ("extractor", Some(10)),
                ("foundry", Some(20)),
                ("shipyard", None),
            ] {
                facilities.push(LivingFacilityV2 {
                    id: allocate(),
                    world,
                    definition: slug(definition)?,
                    status: LivingFacilityStatusV2::Operational,
                    disabled: false,
                    paid_alloy: 0,
                    hit_points: 0,
                    next_due: due.map(LivingTickV2),
                    next_repair: None,
                    blocked_reasons: LivingSortedVecV2::default(),
                });
            }
            let escort_count = if index < 2 { 4 } else { 1 };
            let mut hulls = Vec::new();
            for _ in 0..escort_count {
                hulls.push(LivingHullV2 {
                    id: allocate(),
                    definition: slug("escort")?,
                    hit_points: 10,
                    return_credit: false,
                    next_repair: None,
                });
            }
            hulls.push(LivingHullV2 {
                id: allocate(),
                definition: slug("scout")?,
                hit_points: 10,
                return_credit: false,
                next_repair: None,
            });
            fleets.push(LivingFleetV2 {
                id: allocate(),
                owner,
                location: LivingFleetLocationV2::Docked { world },
                order: None,
                hulls: rows(hulls)?,
            });
        }
    }
    if curated {
        for a in 0..4 {
            for b in a + 1..4 {
                let (system_a, system_b) = ordered(systems[a].id, systems[b].id);
                lanes.push(LivingLaneV2 {
                    id: allocate(),
                    system_a,
                    system_b,
                    distance_units: if a == 0 && b == 2 { 100 } else { 250 },
                });
            }
        }
    } else if system_count > 1 {
        for a in 0..system_count {
            let (system_a, system_b) = ordered(systems[a].id, systems[(a + 1) % system_count].id);
            lanes.push(LivingLaneV2 {
                id: allocate(),
                system_a,
                system_b,
                distance_units: 100 + ((a as u64).wrapping_mul(seed.wrapping_add(1)) % 200),
            });
        }
    }
    state.systems = rows(systems)?;
    state.stars = rows(stars)?;
    state.worlds = rows(worlds)?;
    state.civilizations = rows(civs)?;
    state.colonies = rows(colonies)?;
    state.facilities = rows(facilities)?;
    state.deposits = rows(deposits)?;
    state.lanes = rows(lanes)?;
    state.fleets = rows(fleets)?;
    Ok(state)
}
fn ordered(a: LivingEntityIdV2, b: LivingEntityIdV2) -> (LivingEntityIdV2, LivingEntityIdV2) {
    if a < b { (a, b) } else { (b, a) }
}
fn world_named(state: &LivingGalaxyStateV2, name: &str) -> Result<LivingWorldV2, String> {
    state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.name.as_str() == name)
        .cloned()
        .ok_or_else(|| format!("fixture world {name} absent"))
}
fn configure_commerce(
    state: &mut LivingGalaxyStateV2,
    seed: u64,
    storm: bool,
) -> Result<(), String> {
    let source = world_named(state, "Aster Vale")?;
    let destination = world_named(state, "Crucible")?;
    let sender = source.owner.ok_or("fixture sender absent")?;
    let receiver = destination.owner.ok_or("fixture receiver absent")?;
    state.relations = rows(vec![
        LivingRelationV2 {
            from: sender,
            to: receiver,
            base_disposition: 30,
            reasons: LivingRelationReasonsV2::default(),
        },
        LivingRelationV2 {
            from: receiver,
            to: sender,
            base_disposition: 30,
            reasons: LivingRelationReasonsV2::default(),
        },
    ])?;
    state.routes = rows(vec![LivingFreightRouteV2 {
        id: id(seed, 10000),
        kind: LivingRouteKindV2::Trade,
        source_world: source.id,
        destination_world: destination.id,
        resource: LivingResourceV2::Ore,
        batch_size: 10,
        cadence_ticks: 10,
        source_reserve: 20,
        source_owner: sender,
        receiver_owner: receiver,
        next_due: LivingTickV2(10),
        suspended: false,
    }])?;
    if storm {
        let mut routes = state.routes.as_slice().to_vec();
        routes[0].batch_size = 2;
        routes[0].cadence_ticks = 10;
        routes[0].next_due = LivingTickV2(10);
        state.routes = rows(routes)?;
        let mut worlds = state.worlds.as_slice().to_vec();
        for world in worlds
            .iter_mut()
            .filter(|world| world.owner == Some(receiver))
        {
            world.inventory.ore = 0;
        }
        state.worlds = rows(worlds)?;
        let remaining = state
            .facilities
            .as_slice()
            .iter()
            .filter(|row| {
                !(state
                    .worlds
                    .as_slice()
                    .iter()
                    .any(|world| world.id == row.world && world.owner == Some(receiver))
                    && row.definition.as_str() == "extractor")
            })
            .cloned()
            .collect();
        state.facilities = rows(remaining)?;
        let endpoints = ordered(source.system, destination.system);
        let lane = state
            .lanes
            .as_slice()
            .iter()
            .find(|row| (row.system_a, row.system_b) == endpoints)
            .ok_or("fixture corridor absent")?
            .id;
        state.hazards = rows(vec![LivingHazardV2 {
            id: id(seed, 10001),
            definition: slug("ion_storm")?,
            lane,
            start_tick: LivingTickV2(10),
            end_tick: LivingTickV2(150),
        }])?;
    }
    Ok(())
}
fn configure_frontier(state: &mut LivingGalaxyStateV2) -> Result<(), String> {
    let source = world_named(state, "Aster Vale")?;
    let destination = world_named(state, "Crucible")?;
    let sender = source.owner.ok_or("fixture sender absent")?;
    let receiver = destination.owner.ok_or("fixture receiver absent")?;
    state.relations = rows(vec![
        LivingRelationV2 {
            from: sender,
            to: receiver,
            base_disposition: -40,
            reasons: LivingRelationReasonsV2::default(),
        },
        LivingRelationV2 {
            from: receiver,
            to: sender,
            base_disposition: -40,
            reasons: LivingRelationReasonsV2::default(),
        },
    ])?;
    let mut observations = Vec::new();
    for (civilization, owner) in [(sender, receiver), (receiver, sender)] {
        for world in state
            .worlds
            .as_slice()
            .iter()
            .filter(|row| row.owner == Some(owner))
        {
            let mut hull_counts = std::collections::BTreeMap::<LivingSlugV2, u32>::new();
            for fleet in state
                .fleets
                .as_slice()
                .iter()
                .filter(|row| row.location == LivingFleetLocationV2::Docked { world: world.id })
            {
                for hull in fleet.hulls.as_slice() {
                    *hull_counts.entry(hull.definition.clone()).or_default() += 1;
                }
            }
            let facilities = state
                .facilities
                .as_slice()
                .iter()
                .filter(|row| row.world == world.id)
                .map(|row| LivingObservedFacilityV2 {
                    definition: row.definition.clone(),
                    count: 1,
                })
                .collect();
            observations.push(LivingObservationV2 {
                civilization,
                world: world.id,
                observed_tick: LivingTickV2(0),
                owner: world.owner,
                remaining_deposit: 20000,
                operational_defense_batteries: None,
                facilities: rows(facilities)?,
                stationed_hulls: rows(
                    hull_counts
                        .into_iter()
                        .map(|(definition, count)| LivingObservedHullV2 { definition, count })
                        .collect(),
                )?,
            });
        }
    }
    state.observations = rows(observations)?;
    Ok(())
}
