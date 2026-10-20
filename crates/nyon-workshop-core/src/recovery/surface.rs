//! Integer cube topology ported from `zig/src/surface.zig`.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Face { PositiveX, NegativeX, PositiveY, NegativeY, PositiveZ, NegativeZ }

impl Face {
    pub const ALL: [Self; 6] = [Self::PositiveX, Self::NegativeX, Self::PositiveY, Self::NegativeY, Self::PositiveZ, Self::NegativeZ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction { East, West, North, South }

impl Direction {
    pub const ALL: [Self; 4] = [Self::East, Self::West, Self::North, Self::South];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell { pub face: Face, pub x: u16, pub y: u16 }

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SurfaceError {
    #[error("cube side must be nonzero")]
    InvalidSide,
    #[error("cell lies outside the cube surface")]
    InvalidCell,
}

type Vector = [i32; 3];
// (normal, right, up), in the preserved face order.
const BASES: [(Vector, Vector, Vector); 6] = [
    ([1,0,0], [0,0,-1], [0,1,0]),
    ([-1,0,0], [0,0,1], [0,1,0]),
    ([0,1,0], [1,0,0], [0,0,-1]),
    ([0,-1,0], [1,0,0], [0,0,1]),
    ([0,0,1], [1,0,0], [0,1,0]),
    ([0,0,-1], [-1,0,0], [0,1,0]),
];

fn dot(a: Vector, b: Vector) -> i32 { a[0]*b[0] + a[1]*b[1] + a[2]*b[2] }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Topology { side: u16 }

impl Topology {
    pub fn new(side: u16) -> Result<Self, SurfaceError> {
        if side == 0 { return Err(SurfaceError::InvalidSide); }
        Ok(Self { side })
    }

    pub const fn side(self) -> u16 { self.side }

    pub fn cell_id(self, cell: Cell) -> Result<u64, SurfaceError> {
        self.validate(cell)?;
        let side = u64::from(self.side);
        Ok((cell.face as u64)*side*side + u64::from(cell.y)*side + u64::from(cell.x))
    }

    pub fn from_id(self, id: u64) -> Result<Cell, SurfaceError> {
        let side = u64::from(self.side);
        let area = side*side;
        if id >= 6*area { return Err(SurfaceError::InvalidCell); }
        Ok(Cell { face: Face::ALL[(id/area) as usize], x: (id%area%side) as u16, y: (id%area/side) as u16 })
    }

    fn validate(self, cell: Cell) -> Result<(), SurfaceError> {
        if cell.x >= self.side || cell.y >= self.side { return Err(SurfaceError::InvalidCell); }
        Ok(())
    }

    pub fn neighbor(self, cell: Cell, direction: Direction) -> Result<Cell, SurfaceError> {
        self.validate(cell)?;
        let mut next = cell;
        match direction {
            Direction::East if cell.x < self.side-1 => { next.x += 1; return Ok(next); }
            Direction::West if cell.x > 0 => { next.x -= 1; return Ok(next); }
            Direction::North if cell.y < self.side-1 => { next.y += 1; return Ok(next); }
            Direction::South if cell.y > 0 => { next.y -= 1; return Ok(next); }
            _ => {}
        }
        let (normal, right, up) = BASES[cell.face as usize];
        let mut target_normal = match direction { Direction::East | Direction::West => right, _ => up };
        if matches!(direction, Direction::West | Direction::South) {
            for component in &mut target_normal { *component = -*component; }
        }
        let extent = i32::from(self.side)-1;
        let x = 2*i32::from(cell.x)-extent;
        let y = 2*i32::from(cell.y)-extent;
        let position = std::array::from_fn(|i| normal[i]*extent + right[i]*x + up[i]*y);
        for (index, (normal, right, up)) in BASES.into_iter().enumerate() {
            if dot(normal, target_normal) == 1 {
                return Ok(Cell { face: Face::ALL[index], x: ((dot(position,right)+extent)/2) as u16, y: ((dot(position,up)+extent)/2) as u16 });
            }
        }
        Err(SurfaceError::InvalidCell)
    }
}
