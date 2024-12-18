#![deny(clippy::all)]
#![forbid(unsafe_code)]

use array2d::{Array2D, Error};

const BIRTH_RULE: [bool; 9] = [false, false, false, true, false, false, false, false, false];
const SURVIVE_RULE: [bool; 9] = [false, false, true, true, false, false, false, false, false];
const INITIAL_FILL: f32 = 0.3;

#[derive(Clone, Debug)]
pub struct WorldGrid {
    cells: Array2D<GridCell>,
    scratch_cells: Array2D<GridCell>,
    width: usize,
    height: usize,
    cells1d: Vec<GridCell>,
    // Should always be the same size as `cells`. When updating, we read from
    // `cells` and write to `scratch_cells`, then swap. Otherwise, it's not in
    // use, and `cells` should be updated directly.
    scratch_cells1d: Vec<GridCell>,
}

impl WorldGrid {
    pub fn new_random(width: usize, height: usize) -> Self {
        let mut result = Self::new_empty(width, height);
        result.randomize();
        result
    }

    fn new_empty(width: usize, height: usize) -> Self {
        assert!(width != 0 && height != 0);
        let size = width.checked_mul(height).expect("too big");
        Self {
            cells: Array2D::filled_with(GridCell::default(), height, width),
            scratch_cells: Array2D::filled_with(GridCell::default(), height, width),
            width,
            height,
            cells1d: vec![GridCell::default(); size],
            scratch_cells1d: vec![GridCell::default(); size],
        }
    }

    fn width(&self) -> usize {
        self.cells.num_columns()
    }

    fn height(&self) -> usize {
        self.cells.num_rows()
    }

    pub fn randomize(&mut self) {
        let mut rng: randomize::PCG32 = generate_seed().into();
        for cell in self.cells1d.iter_mut() {
            let alive = randomize::f32_half_open_right(rng.next_u32()) > INITIAL_FILL;
            *cell = GridCell::new(alive);
        }
        // run a few simulation iterations for aesthetics (If we don't, the
        // noise is ugly)
        for _ in 0..3 {
            self.update();
        }
        // Smooth out noise in the heatmap that would remain for a while
        for cell in self.cells1d.iter_mut() {
            cell.cool_off(0.4);
        }
    }

    fn count_neighbors(&self, x: usize, y: usize) -> usize {
        let (xm1, xp1) = if x == 0 {
            (self.width - 1, x + 1)
        } else if x == self.width - 1 {
            (x - 1, 0)
        } else {
            (x - 1, x + 1)
        };
        let (ym1, yp1) = if y == 0 {
            (self.height - 1, y + 1)
        } else if y == self.height - 1 {
            (y - 1, 0)
        } else {
            (y - 1, y + 1)
        };
        self.cells1d[xm1 + ym1 * self.width].alive as usize
            + self.cells1d[x + ym1 * self.width].alive as usize
            + self.cells1d[xp1 + ym1 * self.width].alive as usize
            + self.cells1d[xm1 + y * self.width].alive as usize
            + self.cells1d[xp1 + y * self.width].alive as usize
            + self.cells1d[xm1 + yp1 * self.width].alive as usize
            + self.cells1d[x + yp1 * self.width].alive as usize
            + self.cells1d[xp1 + yp1 * self.width].alive as usize
    }

    pub fn update(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let neighbors = self.count_neighbors(x, y);
                let idx = x + y * self.width;
                let next = self.cells1d[idx].update_neighbors(neighbors);
                // Write into scratch_cells, since we're still reading from `self.cells`
                self.scratch_cells1d[idx] = next;
            }
        }
        std::mem::swap(&mut self.scratch_cells1d, &mut self.cells1d);
    }

    pub fn toggle(&mut self, x: isize, y: isize) -> bool {
        if let Some(i) = self.grid_idx(x, y) {
            let was_alive = self.cells1d[i].alive;
            self.cells1d[i].set_alive(!was_alive);
            !was_alive
        } else {
            false
        }
    }

    pub fn draw(&self, screen: &mut [u8]) {
        debug_assert_eq!(screen.len(), 4 * self.cells1d.len());
        for (cell, pixel) in self.cells1d.iter().zip(screen.chunks_exact_mut(4)) {
            let color_rgba = if cell.alive {
                [0, 0xff, 0xff, 0xff]
            } else {
                [0, 0, cell.heat, 0xff]
            };
            pixel.copy_from_slice(&color_rgba);
        }
    }

    pub fn set_line(&mut self, x0: isize, y0: isize, x1: isize, y1: isize, alive: bool) -> Option<()> {
        // possible to optimize by matching on Clipline and iterating over its arms
        for (x, y) in clipline::Clipline::new(
            ((x0, y0), (x1, y1)),
            ((0, 0), (self.width as isize - 1, self.height as isize - 1)),
        )? {
            let (x, y) = (x as usize, y as usize);
            self.cells1d[x + y * self.width].set_alive(alive);
        }
        Some(())
    }

    fn grid_idx<I: TryInto<usize>>(&self, x: I, y: I) -> Option<usize> {
        match (x.try_into(), y.try_into()) {
            (Ok(x), Ok(y)) if x < self.width && y < self.height => Some(x + y * self.width),
            _ => None,
        }
    }
}

/// Generate a pseudorandom seed for the game's PRNG.
fn generate_seed() -> (u64, u64) {
    use byteorder::{ByteOrder, NativeEndian};
    use getrandom::getrandom;

    let mut seed = [0_u8; 16];

    getrandom(&mut seed).expect("failed to getrandom");

    (
        NativeEndian::read_u64(&seed[0..8]),
        NativeEndian::read_u64(&seed[8..16]),
    )
}

#[derive(Clone, Copy, Debug, Default)]
struct GridCell {
    alive: bool,
    // Used for the trail effect. Always 255 if `self.alive` is true (We could
    // use an enum for Cell, but it makes several functions slightly more
    // complex, and doesn't actually make anything any simpler here, or save any
    // memory, so we don't)
    heat: u8,
}

impl GridCell {
    fn new(alive: bool) -> Self {
        Self { alive, heat: 0 }
    }

    #[must_use]
    fn update_neighbors(self, n: usize) -> Self {
        let next_alive = if self.alive {
            SURVIVE_RULE[n]
        } else {
            BIRTH_RULE[n]
        };
        self.next_state(next_alive)
    }

    #[must_use]
    fn next_state(mut self, alive: bool) -> Self {
        self.alive = alive;
        if self.alive {
            self.heat = 255;
        } else {
            self.heat = self.heat.saturating_sub(1);
        }
        self
    }

    fn set_alive(&mut self, alive: bool) {
        *self = self.next_state(alive);
    }

    fn cool_off(&mut self, decay: f32) {
        if !self.alive {
            let heat = (self.heat as f32 * decay).clamp(0.0, 255.0);
            assert!(heat.is_finite());
            self.heat = heat as u8;
        }
    }
}
