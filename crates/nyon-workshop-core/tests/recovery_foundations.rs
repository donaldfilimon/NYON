//! Preserved source witnesses. Compiled only while behavioral gates are paused.
use nyon_workshop_core::recovery::{
    roles::{AdmissionError, CivilizationId, Provenance, Role, SessionPolicy},
    surface::{Cell, Direction, Face, SurfaceError, Topology},
};

#[test]
fn role_admission_preserves_provenance_and_rejection_precedence() {
    let faction = CivilizationId(7);
    let other = CivilizationId(8);
    let creator = SessionPolicy::new(Role::Creator, None).unwrap();
    let ruler = SessionPolicy::new(Role::Ruler, Some(faction)).unwrap();
    let combined = SessionPolicy::new(Role::Combined, Some(faction)).unwrap();
    assert_eq!(creator.admit(Provenance::Creator, None), Ok(()));
    assert_eq!(ruler.admit(Provenance::Ruler, Some(faction)), Ok(()));
    assert_eq!(combined.admit(Provenance::Creator, None), Ok(()));
    assert_eq!(combined.admit(Provenance::Ruler, Some(faction)), Ok(()));
    assert_eq!(creator.admit(Provenance::Ruler, Some(faction)), Err(AdmissionError::RulerForbidden));
    assert_eq!(ruler.admit(Provenance::Creator, Some(faction)), Err(AdmissionError::CreatorForbidden));
    assert_eq!(combined.admit(Provenance::Creator, Some(faction)), Err(AdmissionError::InvalidActor));
    for actor in [None, Some(other)] {
        assert_eq!(combined.admit(Provenance::Ruler, actor), Err(AdmissionError::WrongCivilization));
    }
    for policy in [creator, ruler, combined] {
        assert_eq!(policy.admit(Provenance::Autonomous, Some(faction)), Err(AdmissionError::InternalCommandOnly));
    }
    for role in [Role::Ruler, Role::Combined] {
        assert_eq!(SessionPolicy::new(role, None), Err(AdmissionError::CivilizationRequired));
    }
}

#[test]
fn cube_seams_match_independent_positive_z_witnesses() {
    let topology = Topology::new(4).unwrap();
    assert_eq!(topology.neighbor(Cell { face: Face::PositiveZ, x: 3, y: 1 }, Direction::East),
        Ok(Cell { face: Face::PositiveX, x: 0, y: 1 }));
    assert_eq!(topology.neighbor(Cell { face: Face::PositiveZ, x: 2, y: 3 }, Direction::North),
        Ok(Cell { face: Face::PositiveY, x: 2, y: 0 }));
    assert_eq!(Topology::new(0), Err(SurfaceError::InvalidSide));
    assert_eq!(topology.from_id(96), Err(SurfaceError::InvalidCell));
    assert_eq!(topology.neighbor(Cell { face: Face::PositiveZ, x: 4, y: 0 }, Direction::East), Err(SurfaceError::InvalidCell));
}

#[test]
fn all_cube_cells_roundtrip_and_have_reciprocal_neighbors() {
    for side in [1, 2, 3, 8] {
        let topology = Topology::new(side).unwrap();
        for id in 0..6*u64::from(side)*u64::from(side) {
            let cell = topology.from_id(id).unwrap();
            assert_eq!(topology.cell_id(cell), Ok(id));
            for direction in Direction::ALL {
                let next = topology.neighbor(cell, direction).unwrap();
                let reverse_count = Direction::ALL.into_iter()
                    .filter(|&reverse| topology.neighbor(next, reverse) == Ok(cell)).count();
                assert_eq!(reverse_count, 1);
            }
        }
    }
}

#[test]
fn maximum_cube_side_retains_wide_integer_ids() {
    let topology = Topology::new(u16::MAX).unwrap();
    let last = 6*u64::from(u16::MAX)*u64::from(u16::MAX)-1;
    assert_eq!(topology.cell_id(topology.from_id(last).unwrap()), Ok(last));
    for face in Face::ALL {
        let corner = Cell { face, x: u16::MAX-1, y: u16::MAX-1 };
        for direction in Direction::ALL {
            topology.cell_id(topology.neighbor(corner, direction).unwrap()).unwrap();
        }
    }
}
