use minesweeper_rust::game::{CellState, Difficulty, Game, GameStatus};
use std::collections::HashSet;

fn coord_set(coords: &[(usize, usize)]) -> HashSet<(usize, usize)> {
    coords.iter().copied().collect()
}

fn neighbours(rows: usize, cols: usize, row: usize, col: usize) -> Vec<(usize, usize)> {
    let mut result = Vec::with_capacity(8);
    let row_start = row.saturating_sub(1);
    let row_end = row.saturating_add(1).min(rows - 1);
    let col_start = col.saturating_sub(1);
    let col_end = col.saturating_add(1).min(cols - 1);
    for neighbour_row in row_start..=row_end {
        for neighbour_col in col_start..=col_end {
            if neighbour_row != row || neighbour_col != col {
                result.push((neighbour_row, neighbour_col));
            }
        }
    }
    result
}

fn states(game: &Game) -> Vec<CellState> {
    let mut result = Vec::with_capacity(game.rows() * game.cols());
    for row in 0..game.rows() {
        for col in 0..game.cols() {
            result.push(game.cell(row, col).expect("valid cell").state);
        }
    }
    result
}

fn hidden_safe_neighbours(game: &Game, row: usize, col: usize) -> Vec<(usize, usize)> {
    neighbours(game.rows(), game.cols(), row, col)
        .into_iter()
        .filter(|&(neighbour_row, neighbour_col)| {
            let cell = game
                .cell(neighbour_row, neighbour_col)
                .expect("valid neighbour");
            cell.state == CellState::Hidden && !cell.mine
        })
        .collect()
}

fn hidden_mine_neighbours(game: &Game, row: usize, col: usize) -> Vec<(usize, usize)> {
    neighbours(game.rows(), game.cols(), row, col)
        .into_iter()
        .filter(|&(neighbour_row, neighbour_col)| {
            let cell = game
                .cell(neighbour_row, neighbour_col)
                .expect("valid neighbour");
            cell.state == CellState::Hidden && cell.mine
        })
        .collect()
}

/// Builds a deterministic started game that has a revealed numbered cell with
/// at least one hidden safe neighbour and at least one adjacent hidden mine.
fn started_game_with_chord_target() -> (Game, (usize, usize)) {
    for seed in 0..1_000u64 {
        let mut game = Game::new(Difficulty::Beginner, seed);
        assert!(game.reveal(4, 4));
        if game.status() != GameStatus::Playing {
            continue;
        }

        for row in 0..game.rows() {
            for col in 0..game.cols() {
                let cell = game.cell(row, col).expect("valid cell");
                if cell.state != CellState::Revealed || cell.adjacent == 0 {
                    continue;
                }
                if !hidden_safe_neighbours(&game, row, col).is_empty()
                    && !hidden_mine_neighbours(&game, row, col).is_empty()
                {
                    return (game, (row, col));
                }
            }
        }
    }
    panic!("failed to find a deterministic chord opportunity");
}

fn expected_flood(game: &Game, start: (usize, usize)) -> HashSet<(usize, usize)> {
    let mut visited = HashSet::new();
    let mut stack = vec![start];
    while let Some((row, col)) = stack.pop() {
        if !visited.insert((row, col)) {
            continue;
        }

        let cell = game.cell(row, col).expect("valid flood cell");
        assert!(!cell.mine, "the first click must be safe");
        if cell.adjacent == 0 {
            for (neighbour_row, neighbour_col) in neighbours(game.rows(), game.cols(), row, col) {
                let neighbour = game
                    .cell(neighbour_row, neighbour_col)
                    .expect("valid neighbour");
                if !neighbour.mine {
                    stack.push((neighbour_row, neighbour_col));
                }
            }
        }
    }
    visited
}

#[test]
fn difficulty_metadata_and_ready_state_are_correct() {
    assert_eq!(
        Difficulty::ALL,
        [
            Difficulty::Beginner,
            Difficulty::Intermediate,
            Difficulty::Expert
        ]
    );

    let expected = [
        (
            Difficulty::Beginner,
            (9, 9),
            10,
            "\u{041D}\u{043E}\u{0432}\u{0438}\u{0447}\u{043E}\u{043A}",
        ),
        (
            Difficulty::Intermediate,
            (16, 16),
            40,
            "\u{041B}\u{044E}\u{0431}\u{0438}\u{0442}\u{0435}\u{043B}\u{044C}",
        ),
        (
            Difficulty::Expert,
            (16, 30),
            99,
            "\u{042D}\u{043A}\u{0441}\u{043F}\u{0435}\u{0440}\u{0442}",
        ),
    ];

    for (difficulty, dimensions, mine_count, label) in expected {
        assert_eq!(difficulty.dimensions(), dimensions);
        assert_eq!(difficulty.mine_count(), mine_count);
        assert_eq!(difficulty.label_ru(), label);

        let game = Game::new(difficulty, 0);
        assert_eq!(game.difficulty(), difficulty);
        assert_eq!(game.rows(), dimensions.0);
        assert_eq!(game.cols(), dimensions.1);
        assert_eq!(game.mine_count(), mine_count);
        assert_eq!(game.status(), GameStatus::Ready);
        assert_eq!(
            game.flags_remaining(),
            i32::try_from(mine_count).expect("mine count fits i32")
        );
        assert!(game.mine_positions().is_empty());

        for row in 0..difficulty.dimensions().0 {
            for col in 0..difficulty.dimensions().1 {
                let cell = game.cell(row, col).expect("valid cell");
                assert_eq!(cell.state, CellState::Hidden);
                assert_eq!(cell.adjacent, 0);
                assert!(!cell.mine);
            }
        }
    }
}

#[test]
fn mine_placement_is_deterministic_and_adjacency_counts_are_exact() {
    let mut first = Game::new(Difficulty::Beginner, 0);
    let mut second = Game::new(Difficulty::Beginner, 0);
    assert!(first.reveal(4, 4));
    assert!(second.reveal(4, 4));

    assert_eq!(first.status(), second.status());
    assert_eq!(first.flags_remaining(), second.flags_remaining());
    let first_mines = first.mine_positions();
    let second_mines = second.mine_positions();
    assert_eq!(first_mines, second_mines);
    assert_eq!(first_mines.len(), Difficulty::Beginner.mine_count());
    assert_eq!(
        first_mines.iter().copied().collect::<HashSet<_>>().len(),
        Difficulty::Beginner.mine_count()
    );

    for row in 0..first.rows() {
        for col in 0..first.cols() {
            assert_eq!(first.cell(row, col), second.cell(row, col));
        }
    }

    let mines = coord_set(&first_mines);
    for row in 0..first.rows() {
        for col in 0..first.cols() {
            let cell = first.cell(row, col).expect("valid cell");
            assert_eq!(cell.mine, mines.contains(&(row, col)));
            if !cell.mine {
                let expected_adjacent = neighbours(first.rows(), first.cols(), row, col)
                    .into_iter()
                    .filter(|coordinate| mines.contains(coordinate))
                    .count();
                assert_eq!(usize::from(cell.adjacent), expected_adjacent);
            }
        }
    }

    let mut different = Game::new(Difficulty::Beginner, 1);
    assert!(different.reveal(4, 4));
    assert_ne!(different.mine_positions(), first_mines);

    let mut zero_seed = Game::new(Difficulty::Beginner, 0);
    assert!(zero_seed.reveal(4, 4));
    assert_eq!(zero_seed.mine_positions(), first_mines);
}

#[test]
fn first_click_and_its_3x3_neighbourhood_are_always_safe() {
    for difficulty in Difficulty::ALL {
        let (rows, cols) = difficulty.dimensions();
        for click_row in 0..rows {
            for click_col in 0..cols {
                let mut game = Game::new(difficulty, 0xC0DE);
                assert!(game.reveal(click_row, click_col));

                let clicked = game.cell(click_row, click_col).expect("valid click");
                assert_eq!(clicked.state, CellState::Revealed);
                assert!(!clicked.mine);
                assert_eq!(
                    clicked.adjacent, 0,
                    "first click must border only safe cells"
                );
                assert!(matches!(
                    game.status(),
                    GameStatus::Playing | GameStatus::Won
                ));

                for (neighbour_row, neighbour_col) in neighbours(rows, cols, click_row, click_col) {
                    assert_eq!(
                        game.cell(neighbour_row, neighbour_col)
                            .expect("valid neighbour")
                            .state,
                        CellState::Revealed,
                        "the opening flood should reveal the safe 3x3 zone"
                    );
                }

                let mines = game.mine_positions();
                assert_eq!(mines.len(), difficulty.mine_count());
                for (mine_row, mine_col) in mines {
                    assert!(
                        mine_row.abs_diff(click_row) > 1 || mine_col.abs_diff(click_col) > 1,
                        "mine at ({mine_row}, {mine_col}) violates the safe 3x3 zone"
                    );
                }

                let revealed_count = (0..rows)
                    .flat_map(|row| (0..cols).map(move |col| (row, col)))
                    .filter(|&(row, col)| {
                        game.cell(row, col)
                            .is_some_and(|cell| cell.state == CellState::Revealed)
                    })
                    .count();
                assert!(revealed_count > 1, "opening click should flood-fill");
            }
        }
    }
}

#[test]
fn flood_fill_opens_the_connected_zero_region_and_its_numbered_border() {
    let mut game = Game::new(Difficulty::Beginner, 0x5EED);
    let start = (4, 4);
    assert!(game.reveal(start.0, start.1));
    assert_eq!(game.status(), GameStatus::Playing);

    let expected = expected_flood(&game, start);
    let mut actual = HashSet::new();
    for row in 0..game.rows() {
        for col in 0..game.cols() {
            let cell = game.cell(row, col).expect("valid cell");
            if cell.state == CellState::Revealed {
                assert!(!cell.mine, "flood fill must not reveal a mine");
                actual.insert((row, col));
            }
        }
    }

    assert_eq!(actual, expected);
    assert!(expected.len() > 1);

    for &(row, col) in &expected {
        let cell = game.cell(row, col).expect("valid cell");
        assert_eq!(cell.state, CellState::Revealed);
        if cell.adjacent == 0 {
            for (neighbour_row, neighbour_col) in neighbours(game.rows(), game.cols(), row, col) {
                let neighbour = game
                    .cell(neighbour_row, neighbour_col)
                    .expect("valid neighbour");
                if !neighbour.mine {
                    assert_eq!(neighbour.state, CellState::Revealed);
                }
            }
        }
    }

    for (row, col) in neighbours(game.rows(), game.cols(), start.0, start.1) {
        assert_eq!(
            game.cell(row, col).expect("valid neighbour").state,
            CellState::Revealed
        );
    }
}

#[test]
fn flags_toggle_cap_at_mine_count_and_never_flag_revealed_cells() {
    let mut game = Game::new(Difficulty::Beginner, 7);
    let mine_count = game.mine_count();
    assert_eq!(game.status(), GameStatus::Ready);

    for index in 0..mine_count {
        let row = index / game.cols();
        let col = index % game.cols();
        assert!(game.toggle_flag(row, col));
        assert_eq!(
            game.cell(row, col).expect("valid cell").state,
            CellState::Flagged
        );
        assert_eq!(
            game.status(),
            GameStatus::Ready,
            "flags do not start the game"
        );
        assert!(game.mine_positions().is_empty(), "mines remain lazy");
    }
    assert_eq!(game.flags_remaining(), 0);

    let extra_index = mine_count;
    let extra = (extra_index / game.cols(), extra_index % game.cols());
    assert!(!game.toggle_flag(extra.0, extra.1));
    assert_eq!(
        game.cell(extra.0, extra.1).expect("valid cell").state,
        CellState::Hidden
    );
    assert_eq!(game.flags_remaining(), 0);

    assert!(game.toggle_flag(0, 0));
    assert_eq!(game.flags_remaining(), 1);
    assert!(game.toggle_flag(0, 0));
    assert_eq!(game.flags_remaining(), 0);

    let opening = (8, 8);
    assert!(game.reveal(opening.0, opening.1));
    assert!(matches!(
        game.status(),
        GameStatus::Playing | GameStatus::Won
    ));
    let opened = game.cell(opening.0, opening.1).expect("valid cell");
    assert_eq!(opened.state, CellState::Revealed);
    assert!(!game.toggle_flag(opening.0, opening.1));
    assert_eq!(
        game.cell(opening.0, opening.1).expect("valid cell").state,
        CellState::Revealed
    );

    let flagged_count = (0..game.rows())
        .flat_map(|row| (0..game.cols()).map(move |col| (row, col)))
        .filter(|&(row, col)| {
            game.cell(row, col)
                .is_some_and(|cell| cell.state == CellState::Flagged)
        })
        .count();
    assert_eq!(flagged_count, mine_count);
    assert_eq!(game.flags_remaining(), 0);
}

#[test]
fn winning_reveals_every_safe_cell_and_auto_flags_all_mines() {
    let mut game = Game::new(Difficulty::Beginner, 0x7777);
    assert!(game.reveal(4, 4));
    let mines = coord_set(&game.mine_positions());
    assert_eq!(mines.len(), game.mine_count());

    for row in 0..game.rows() {
        for col in 0..game.cols() {
            if !mines.contains(&(row, col))
                && game.status() == GameStatus::Playing
                && game.cell(row, col).expect("valid cell").state == CellState::Hidden
            {
                assert!(game.reveal(row, col));
            }
        }
    }

    assert_eq!(game.status(), GameStatus::Won);
    assert_eq!(game.flags_remaining(), 0);
    assert_eq!(coord_set(&game.mine_positions()), mines);

    for row in 0..game.rows() {
        for col in 0..game.cols() {
            let cell = game.cell(row, col).expect("valid cell");
            assert_eq!(cell.mine, mines.contains(&(row, col)));
            if cell.mine {
                assert_eq!(cell.state, CellState::Flagged);
            } else {
                assert_eq!(cell.state, CellState::Revealed);
            }
        }
    }

    let target = (0, 0);
    assert!(!game.reveal(target.0, target.1));
    assert!(!game.toggle_flag(target.0, target.1));
    assert!(!game.chord(target.0, target.1));
}

#[test]
fn losing_changes_only_status_and_terminal_input_is_ignored() {
    let mut game = Game::new(Difficulty::Beginner, 0x1234);
    assert!(game.reveal(4, 4));
    assert_eq!(game.status(), GameStatus::Playing);

    let mines = game.mine_positions();
    assert_eq!(mines.len(), game.mine_count());
    let (mine_row, mine_col) = mines[0];
    assert_eq!(
        game.cell(mine_row, mine_col).expect("valid mine").state,
        CellState::Hidden
    );
    let before_states = states(&game);

    assert!(game.reveal(mine_row, mine_col));
    assert_eq!(game.status(), GameStatus::Lost);
    assert_eq!(
        states(&game),
        before_states,
        "a loss must not reveal cells automatically"
    );
    assert_eq!(
        game.cell(mine_row, mine_col).expect("valid mine").state,
        CellState::Hidden
    );

    for (row, col) in game.mine_positions() {
        let cell = game.cell(row, col).expect("valid mine");
        assert!(cell.mine);
    }

    let hidden_safe = (0..game.rows())
        .flat_map(|row| (0..game.cols()).map(move |col| (row, col)))
        .find(|&(row, col)| {
            let cell = game.cell(row, col).expect("valid cell");
            cell.state == CellState::Hidden && !cell.mine
        })
        .expect("a playing game has a hidden safe cell");
    let lost_states = states(&game);
    assert!(!game.reveal(hidden_safe.0, hidden_safe.1));
    assert!(!game.toggle_flag(mine_row, mine_col));
    assert!(!game.chord(hidden_safe.0, hidden_safe.1));
    assert_eq!(states(&game), lost_states);
}

#[test]
fn chord_with_matching_flags_reveals_all_hidden_neighbours() {
    let (mut game, target) = started_game_with_chord_target();
    let target_cell = game.cell(target.0, target.1).expect("valid target");
    assert_eq!(target_cell.state, CellState::Revealed);
    assert!(target_cell.adjacent > 0);

    let hidden_safe = hidden_safe_neighbours(&game, target.0, target.1);
    let hidden_mines = hidden_mine_neighbours(&game, target.0, target.1);
    assert!(!hidden_safe.is_empty());
    assert_eq!(hidden_mines.len(), usize::from(target_cell.adjacent));

    assert!(
        !game.chord(target.0, target.1),
        "no flags means count mismatch"
    );
    for &(row, col) in &hidden_mines {
        assert!(game.toggle_flag(row, col));
    }
    assert_eq!(
        game.cell(target.0, target.1).expect("valid target").state,
        CellState::Revealed
    );

    assert!(game.chord(target.0, target.1));
    assert!(matches!(
        game.status(),
        GameStatus::Playing | GameStatus::Won
    ));

    for &(row, col) in &hidden_safe {
        assert_eq!(
            game.cell(row, col).expect("valid safe neighbour").state,
            CellState::Revealed
        );
    }
    for &(row, col) in &hidden_mines {
        assert_eq!(
            game.cell(row, col).expect("valid mine neighbour").state,
            CellState::Flagged
        );
    }
}

#[test]
fn chord_with_a_wrong_flag_loses_without_automatic_cell_reveals() {
    let (mut game, target) = started_game_with_chord_target();
    let target_cell = game.cell(target.0, target.1).expect("valid target");
    let hidden_safe = hidden_safe_neighbours(&game, target.0, target.1);
    let mut hidden_mines = hidden_mine_neighbours(&game, target.0, target.1);
    assert!(!hidden_safe.is_empty());
    assert!(!hidden_mines.is_empty());
    assert_eq!(hidden_mines.len(), usize::from(target_cell.adjacent));

    let wrong_flag = hidden_safe[0];
    let unflagged_mine = hidden_mines.pop().expect("at least one adjacent mine");
    assert!(game.toggle_flag(wrong_flag.0, wrong_flag.1));
    for &(row, col) in &hidden_mines {
        assert!(game.toggle_flag(row, col));
    }

    let flagged_total = hidden_mines.len() + 1;
    assert_eq!(flagged_total, usize::from(target_cell.adjacent));
    let before_states = states(&game);
    assert!(game.chord(target.0, target.1));
    assert_eq!(game.status(), GameStatus::Lost);
    assert_eq!(
        states(&game),
        before_states,
        "a losing chord must not reveal cells automatically"
    );
    assert_eq!(
        game.cell(unflagged_mine.0, unflagged_mine.1)
            .expect("valid mine")
            .state,
        CellState::Hidden
    );
    assert_eq!(
        game.cell(wrong_flag.0, wrong_flag.1)
            .expect("valid wrong flag")
            .state,
        CellState::Flagged
    );
}

#[test]
fn reset_and_set_difficulty_create_fresh_deterministic_boards() {
    let mut game = Game::new(Difficulty::Beginner, 111);
    assert!(game.reveal(4, 4));
    let old_mines = game.mine_positions();
    assert!(!old_mines.is_empty());
    assert!(game.toggle_flag(0, 0));

    game.reset(222);
    assert_eq!(game.difficulty(), Difficulty::Beginner);
    assert_eq!((game.rows(), game.cols()), (9, 9));
    assert_eq!(game.mine_count(), 10);
    assert_eq!(game.status(), GameStatus::Ready);
    assert_eq!(game.flags_remaining(), 10);
    assert!(game.mine_positions().is_empty());
    for row in 0..game.rows() {
        for col in 0..game.cols() {
            let cell = game.cell(row, col).expect("valid cell");
            assert_eq!(cell.state, CellState::Hidden);
            assert_eq!(cell.adjacent, 0);
            assert!(!cell.mine);
        }
    }

    assert!(game.reveal(4, 4));
    let mut expected = Game::new(Difficulty::Beginner, 222);
    assert!(expected.reveal(4, 4));
    assert_eq!(game.mine_positions(), expected.mine_positions());
    assert_eq!(states(&game), states(&expected));

    game.set_difficulty(Difficulty::Expert, 333);
    assert_eq!(game.difficulty(), Difficulty::Expert);
    assert_eq!((game.rows(), game.cols()), (16, 30));
    assert_eq!(game.mine_count(), 99);
    assert_eq!(game.status(), GameStatus::Ready);
    assert_eq!(game.flags_remaining(), 99);
    assert!(game.mine_positions().is_empty());

    assert!(game.reveal(8, 15));
    let mut expert_expected = Game::new(Difficulty::Expert, 333);
    assert!(expert_expected.reveal(8, 15));
    assert_eq!(game.mine_positions(), expert_expected.mine_positions());
    assert_eq!(game.mine_positions().len(), 99);

    game.reset(333);
    assert_eq!(game.difficulty(), Difficulty::Expert);
    assert_eq!(game.status(), GameStatus::Ready);
    assert!(game.mine_positions().is_empty());
}

#[test]
fn out_of_bounds_coordinates_never_panic_or_mutate_the_game() {
    let mut game = Game::new(Difficulty::Expert, 5);
    let invalid = [
        (game.rows(), 0),
        (0, game.cols()),
        (usize::MAX, 0),
        (0, usize::MAX),
        (usize::MAX, usize::MAX),
    ];

    for &(row, col) in &invalid {
        assert!(game.cell(row, col).is_none());
        assert!(!game.reveal(row, col));
        assert!(!game.toggle_flag(row, col));
        assert!(!game.chord(row, col));
    }
    assert_eq!(game.status(), GameStatus::Ready);
    assert_eq!(game.flags_remaining(), 99);
    assert!(game.mine_positions().is_empty());
    assert!(game.cell(game.rows() - 1, game.cols() - 1).is_some());

    assert!(game.reveal(8, 15));
    let before_states = states(&game);
    for &(row, col) in &invalid {
        assert!(!game.reveal(row, col));
        assert!(!game.toggle_flag(row, col));
        assert!(!game.chord(row, col));
    }
    assert_eq!(states(&game), before_states);
    assert_eq!(game.mine_positions().len(), 99);
}
