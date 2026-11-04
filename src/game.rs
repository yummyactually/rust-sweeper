//! Core Minesweeper game engine.
//!
//! The board is deliberately independent of the UI. Mines are generated only
//! once the player makes the first reveal, which guarantees that the opening
//! cell and its surrounding 3x3 area are safe.

/// Difficulty presets exposed by the engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Expert,
}

impl Difficulty {
    pub const ALL: [Difficulty; 3] = [
        Difficulty::Beginner,
        Difficulty::Intermediate,
        Difficulty::Expert,
    ];

    /// Returns `(rows, cols)` for the difficulty.
    pub fn dimensions(self) -> (usize, usize) {
        match self {
            Difficulty::Beginner => (9, 9),
            Difficulty::Intermediate => (16, 16),
            Difficulty::Expert => (16, 30),
        }
    }

    /// Number of mines on the board.
    pub fn mine_count(self) -> usize {
        match self {
            Difficulty::Beginner => 10,
            Difficulty::Intermediate => 40,
            Difficulty::Expert => 99,
        }
    }

    /// Russian display label.
    pub fn label_ru(self) -> &'static str {
        match self {
            Difficulty::Beginner => "\u{041D}\u{043E}\u{0432}\u{0438}\u{0447}\u{043E}\u{043A}",
            Difficulty::Intermediate => {
                "\u{041B}\u{044E}\u{0431}\u{0438}\u{0442}\u{0435}\u{043B}\u{044C}"
            }
            Difficulty::Expert => "\u{042D}\u{043A}\u{0441}\u{043F}\u{0435}\u{0440}\u{0442}",
        }
    }
}

/// Overall state of a game.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameStatus {
    Ready,
    Playing,
    Won,
    Lost,
}

/// Presentation state of one board cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellState {
    Hidden,
    Revealed,
    Flagged,
}

/// Public snapshot of one cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellView {
    pub state: CellState,
    pub adjacent: u8,
    /// Renderers should only expose this to the player after a loss.
    pub mine: bool,
}

/// A deterministic Minesweeper board.
#[derive(Clone, Debug)]
pub struct Game {
    difficulty: Difficulty,
    rows: usize,
    cols: usize,
    mine_count: usize,
    status: GameStatus,
    flags: usize,
    cells: Vec<CellState>,
    mines: Vec<bool>,
    adjacent: Vec<u8>,
    mines_placed: bool,
    rng: XorShift64,
}

impl Game {
    /// Creates a new, unstarted game.
    pub fn new(difficulty: Difficulty, seed: u64) -> Self {
        let (rows, cols) = difficulty.dimensions();
        let size = rows * cols;
        let mine_count = difficulty.mine_count();
        Self {
            difficulty,
            rows,
            cols,
            mine_count,
            status: GameStatus::Ready,
            flags: 0,
            cells: vec![CellState::Hidden; size],
            mines: vec![false; size],
            adjacent: vec![0; size],
            mines_placed: false,
            rng: XorShift64::new(seed),
        }
    }

    /// Starts a fresh board at the current difficulty.
    pub fn reset(&mut self, seed: u64) {
        let difficulty = self.difficulty;
        *self = Game::new(difficulty, seed);
    }

    /// Changes difficulty and starts a fresh board.
    pub fn set_difficulty(&mut self, difficulty: Difficulty, seed: u64) {
        *self = Game::new(difficulty, seed);
    }

    pub fn difficulty(&self) -> Difficulty {
        self.difficulty
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn mine_count(&self) -> usize {
        self.mine_count
    }

    pub fn status(&self) -> GameStatus {
        self.status
    }

    /// Number of mines that can still be flagged.
    pub fn flags_remaining(&self) -> i32 {
        self.mine_count as i32 - self.flags as i32
    }

    /// Returns a safe snapshot of a cell, or `None` for invalid coordinates.
    pub fn cell(&self, row: usize, col: usize) -> Option<CellView> {
        let index = self.index(row, col)?;
        Some(CellView {
            state: self.cells[index],
            adjacent: self.adjacent[index],
            mine: self.mines[index],
        })
    }

    /// Reveals a cell, flood-filling open regions when appropriate.
    ///
    /// Returns `true` if the visible state or the game status changed.
    pub fn reveal(&mut self, row: usize, col: usize) -> bool {
        let Some(index) = self.index(row, col) else {
            return false;
        };

        if !matches!(self.status, GameStatus::Ready | GameStatus::Playing) {
            return false;
        }
        if self.cells[index] != CellState::Hidden {
            return false;
        }

        if !self.mines_placed {
            self.place_mines(row, col);
            self.status = GameStatus::Playing;
        }

        if self.mines[index] {
            // Deliberately do not reveal anything: the UI draws the board from
            // the exposed mine metadata and game status after a loss.
            self.status = GameStatus::Lost;
            return true;
        }

        self.reveal_flood(index);
        self.finish_if_won();
        true
    }

    /// Toggles a flag on a hidden cell while respecting the mine cap.
    pub fn toggle_flag(&mut self, row: usize, col: usize) -> bool {
        let Some(index) = self.index(row, col) else {
            return false;
        };

        if !matches!(self.status, GameStatus::Ready | GameStatus::Playing) {
            return false;
        }

        match self.cells[index] {
            CellState::Hidden if self.flags < self.mine_count => {
                self.cells[index] = CellState::Flagged;
                self.flags += 1;
                true
            }
            CellState::Flagged => {
                self.cells[index] = CellState::Hidden;
                self.flags -= 1;
                true
            }
            CellState::Hidden | CellState::Revealed => false,
        }
    }

    /// Chords a revealed numbered cell.
    ///
    /// Neighbours are opened only when the number of adjacent flags matches the
    /// cell's number. A wrong flag therefore lets an adjacent mine be opened,
    /// resulting in a loss.
    pub fn chord(&mut self, row: usize, col: usize) -> bool {
        let Some(index) = self.index(row, col) else {
            return false;
        };

        if self.status != GameStatus::Playing {
            return false;
        }
        if self.cells[index] != CellState::Revealed || self.adjacent[index] == 0 {
            return false;
        }

        let mut adjacent_flags = 0usize;
        let mut hidden_neighbours = Vec::with_capacity(8);
        self.for_each_neighbour(index, |neighbour| match self.cells[neighbour] {
            CellState::Flagged => adjacent_flags += 1,
            CellState::Hidden => hidden_neighbours.push(neighbour),
            CellState::Revealed => {}
        });

        if adjacent_flags != self.adjacent[index] as usize || hidden_neighbours.is_empty() {
            return false;
        }

        // Do not reveal any cells if this chord contains an unflagged mine:
        // losing exposes no engine-side cell automatically.
        if hidden_neighbours
            .iter()
            .any(|&neighbour| self.mines[neighbour])
        {
            self.status = GameStatus::Lost;
            return true;
        }

        let mut changed = false;
        for neighbour in hidden_neighbours {
            if self.cells[neighbour] == CellState::Hidden {
                changed |= self.reveal_flood(neighbour);
            }
        }

        if changed {
            self.finish_if_won();
        }
        changed
    }

    /// Mine coordinates in row-major order.
    ///
    /// This is empty until the first reveal has caused lazy mine placement.
    pub fn mine_positions(&self) -> Vec<(usize, usize)> {
        self.mines
            .iter()
            .enumerate()
            .filter_map(|(index, &is_mine)| {
                if is_mine {
                    Some((index / self.cols, index % self.cols))
                } else {
                    None
                }
            })
            .collect()
    }

    fn index(&self, row: usize, col: usize) -> Option<usize> {
        if row < self.rows && col < self.cols {
            Some(row * self.cols + col)
        } else {
            None
        }
    }

    fn place_mines(&mut self, safe_row: usize, safe_col: usize) {
        let mut candidates = Vec::with_capacity(self.rows * self.cols - self.mine_count);
        for row in 0..self.rows {
            for col in 0..self.cols {
                let in_safe_zone = row.abs_diff(safe_row) <= 1 && col.abs_diff(safe_col) <= 1;
                if !in_safe_zone {
                    candidates.push(row * self.cols + col);
                }
            }
        }

        // Partial Fisher-Yates shuffle: uniformly choose the first mine_count
        // entries while retaining deterministic xorshift output.
        for upper in (1..candidates.len()).rev() {
            let swap_index = self.rng.next_bounded(upper + 1);
            candidates.swap(upper, swap_index);
        }

        for &mine_index in candidates.iter().take(self.mine_count) {
            self.mines[mine_index] = true;
        }
        self.compute_adjacent_counts();
        self.mines_placed = true;
    }

    fn compute_adjacent_counts(&mut self) {
        self.adjacent.fill(0);
        for index in 0..self.mines.len() {
            if !self.mines[index] {
                continue;
            }

            let row = index / self.cols;
            let col = index % self.cols;
            let row_start = row.saturating_sub(1);
            let row_end = (row + 1).min(self.rows - 1);
            let col_start = col.saturating_sub(1);
            let col_end = (col + 1).min(self.cols - 1);

            for neighbour_row in row_start..=row_end {
                for neighbour_col in col_start..=col_end {
                    if neighbour_row == row && neighbour_col == col {
                        continue;
                    }
                    let neighbour = neighbour_row * self.cols + neighbour_col;
                    self.adjacent[neighbour] = self.adjacent[neighbour].saturating_add(1);
                }
            }
        }
    }
    fn reveal_flood(&mut self, start: usize) -> bool {
        if self.cells[start] != CellState::Hidden {
            return false;
        }

        let mut changed = false;
        let mut stack = vec![start];
        while let Some(index) = stack.pop() {
            if self.cells[index] != CellState::Hidden {
                continue;
            }

            self.cells[index] = CellState::Revealed;
            changed = true;

            if self.adjacent[index] == 0 {
                self.for_each_neighbour(index, |neighbour| {
                    if self.cells[neighbour] == CellState::Hidden {
                        stack.push(neighbour);
                    }
                });
            }
        }
        changed
    }

    fn finish_if_won(&mut self) -> bool {
        let all_safe_revealed = self
            .mines
            .iter()
            .enumerate()
            .all(|(index, &is_mine)| is_mine || self.cells[index] == CellState::Revealed);
        if !all_safe_revealed {
            return false;
        }

        self.status = GameStatus::Won;
        self.flags = 0;
        for index in 0..self.mines.len() {
            if self.mines[index] {
                self.cells[index] = CellState::Flagged;
                self.flags += 1;
            }
        }
        true
    }

    fn for_each_neighbour(&self, index: usize, mut visit: impl FnMut(usize)) {
        let row = index / self.cols;
        let col = index % self.cols;
        let row_start = row.saturating_sub(1);
        let row_end = (row + 1).min(self.rows - 1);
        let col_start = col.saturating_sub(1);
        let col_end = (col + 1).min(self.cols - 1);

        for neighbour_row in row_start..=row_end {
            for neighbour_col in col_start..=col_end {
                if neighbour_row == row && neighbour_col == col {
                    continue;
                }
                visit(neighbour_row * self.cols + neighbour_col);
            }
        }
    }
}

/// Small, dependency-free xorshift64* PRNG.
#[derive(Clone, Debug)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        // Fold in a nonzero constant so seed zero remains useful.
        let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
        if state == 0 {
            state = 0xA5A5_5A5A_0123_4567;
        }
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Unbiased-enough bounded sample using the high half of a 128-bit product.
    fn next_bounded(&mut self, upper: usize) -> usize {
        debug_assert!(upper > 0);
        ((u128::from(self.next_u64()) * upper as u128) >> 64) as usize
    }
}
