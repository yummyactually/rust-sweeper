//! Classic Windows Minesweeper presentation: native-menu state, layout,
//! hit-testing, timer, and GDI drawing.

use crate::win32::{
    self, Canvas, Color, Event, EventResult, Font, MenuState, Point, Rect, WindowApp, DT_CENTER,
    DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER,
};
use minesweeper_rust::game::{CellState, CellView, Difficulty, Game, GameStatus};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TITLE: &str = "\u{0421}\u{0430}\u{043f}\u{0451}\u{0440}";

const CELL_SIZE: i32 = 30;
const BOARD_FRAME: i32 = 6;
const PANEL_HEIGHT: i32 = 60;
const PANEL_GAP: i32 = 10;
const WINDOW_MARGIN: i32 = 10;

const LED_WIDTH: i32 = 64;
const LED_HEIGHT: i32 = 38;
const DIGIT_WIDTH: i32 = 16;
const DIGIT_HEIGHT: i32 = 28;
const DIGIT_GAP: i32 = 4;
const SEGMENT: i32 = 3;
const SMILEY_SIZE: i32 = 42;

const BACKGROUND: Color = win32::rgb(192, 192, 192);
const WHITE: Color = win32::rgb(255, 255, 255);
const FACE_DARK: Color = win32::rgb(128, 128, 128);
const BLACK: Color = win32::rgb(0, 0, 0);
const RED: Color = win32::rgb(255, 0, 0);
const DARK_RED: Color = win32::rgb(128, 0, 0);
const LED_ON: Color = win32::rgb(255, 0, 0);
const LED_OFF: Color = win32::rgb(28, 0, 0);
const YELLOW: Color = win32::rgb(255, 255, 0);
const NUMBER_1: Color = win32::rgb(0, 0, 255);
const NUMBER_2: Color = win32::rgb(0, 128, 0);
const NUMBER_3: Color = win32::rgb(255, 0, 0);
const NUMBER_4: Color = win32::rgb(0, 0, 128);
const NUMBER_5: Color = win32::rgb(128, 0, 0);
const NUMBER_6: Color = win32::rgb(0, 128, 128);
const NUMBER_7: Color = win32::rgb(0, 0, 0);
const NUMBER_8: Color = win32::rgb(128, 128, 128);

static SEED_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Pressed {
    #[default]
    None,
    Cell(usize, usize),
    Smiley,
    Elsewhere,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FaceState {
    Normal,
    Surprised,
    Won,
    Lost,
}

#[derive(Clone, Copy, Debug)]
struct ClassicLayout {
    panel: Rect,
    board: Rect,
    grid: Rect,
    led_left: Rect,
    led_right: Rect,
    smiley: Rect,
    rows: usize,
    cols: usize,
}

impl ClassicLayout {
    fn new(difficulty: Difficulty, width: i32, height: i32) -> Self {
        let (rows, cols) = difficulty.dimensions();
        let content_width = cols as i32 * CELL_SIZE + BOARD_FRAME * 2;
        let content_height = rows as i32 * CELL_SIZE + BOARD_FRAME * 2;
        let content_x = (width - content_width) / 2;
        let panel = Rect::from_xywh(content_x, WINDOW_MARGIN, content_width, PANEL_HEIGHT);
        let board = Rect::from_xywh(
            content_x,
            panel.bottom + PANEL_GAP,
            content_width,
            content_height,
        );
        let grid = Rect::new(
            board.left + BOARD_FRAME,
            board.top + BOARD_FRAME,
            board.right - BOARD_FRAME,
            board.bottom - BOARD_FRAME,
        );
        let led_y = panel.center_y() - LED_HEIGHT / 2;
        let led_left = Rect::from_xywh(panel.left + 8, led_y, LED_WIDTH, LED_HEIGHT);
        let led_right = Rect::from_xywh(panel.right - 8 - LED_WIDTH, led_y, LED_WIDTH, LED_HEIGHT);
        let smiley = Rect::from_xywh(
            panel.center_x() - SMILEY_SIZE / 2,
            panel.center_y() - SMILEY_SIZE / 2,
            SMILEY_SIZE,
            SMILEY_SIZE,
        );
        let _ = height;
        Self {
            panel,
            board,
            grid,
            led_left,
            led_right,
            smiley,
            rows,
            cols,
        }
    }

    fn cell_rect(self, row: usize, col: usize) -> Rect {
        Rect::from_xywh(
            self.grid.left + col as i32 * CELL_SIZE,
            self.grid.top + row as i32 * CELL_SIZE,
            CELL_SIZE,
            CELL_SIZE,
        )
    }

    fn cell_at(self, point: Point) -> Option<(usize, usize)> {
        if !self.grid.contains(point) {
            return None;
        }
        let col = (point.x - self.grid.left) / CELL_SIZE;
        let row = (point.y - self.grid.top) / CELL_SIZE;
        if row >= 0 && col >= 0 {
            let row = row as usize;
            let col = col as usize;
            if row < self.rows && col < self.cols {
                return Some((row, col));
            }
        }
        None
    }
}

fn preferred_client_size(difficulty: Difficulty) -> (i32, i32) {
    let (rows, cols) = difficulty.dimensions();
    let content_width = cols as i32 * CELL_SIZE + BOARD_FRAME * 2;
    let content_height = rows as i32 * CELL_SIZE + BOARD_FRAME * 2;
    (
        content_width + WINDOW_MARGIN * 2,
        WINDOW_MARGIN + PANEL_HEIGHT + PANEL_GAP + content_height + WINDOW_MARGIN,
    )
}

struct UiFonts {
    number: Font,
}

impl UiFonts {
    fn new() -> Result<Self, String> {
        Ok(Self {
            number: Font::new("Microsoft Sans Serif", 26, 700, false)?,
        })
    }
}

pub struct App {
    game: Game,
    timer_started: Option<Instant>,
    elapsed: u64,
    questions: Vec<bool>,
    marks_enabled: bool,
    left_down: bool,
    pressed: Pressed,
    exploded_cell: Option<(usize, usize)>,
    fonts: UiFonts,
    window_width: i32,
    window_height: i32,
}

impl App {
    fn new() -> Result<Self, String> {
        let difficulty = Difficulty::Beginner;
        let game = Game::new(difficulty, next_seed());
        let question_count = game.rows().saturating_mul(game.cols());
        let (window_width, window_height) = preferred_client_size(difficulty);
        Ok(Self {
            game,
            timer_started: None,
            elapsed: 0,
            questions: vec![false; question_count],
            marks_enabled: true,
            left_down: false,
            pressed: Pressed::None,
            exploded_cell: None,
            fonts: UiFonts::new()?,
            window_width,
            window_height,
        })
    }

    fn layout(&self) -> ClassicLayout {
        ClassicLayout::new(
            self.game.difficulty(),
            self.window_width,
            self.window_height,
        )
    }

    fn is_terminal(&self) -> bool {
        matches!(self.game.status(), GameStatus::Won | GameStatus::Lost)
    }

    fn menu_difficulty_index(&self) -> usize {
        match self.game.difficulty() {
            Difficulty::Beginner => 0,
            Difficulty::Intermediate => 1,
            Difficulty::Expert => 2,
        }
    }

    fn face_state(&self) -> FaceState {
        match self.game.status() {
            GameStatus::Won => FaceState::Won,
            GameStatus::Lost => FaceState::Lost,
            GameStatus::Ready | GameStatus::Playing => {
                if self.left_down {
                    FaceState::Surprised
                } else {
                    FaceState::Normal
                }
            }
        }
    }

    fn question_at(&self, row: usize, col: usize) -> bool {
        let index = row
            .checked_mul(self.game.cols())
            .and_then(|value| value.checked_add(col));
        index
            .and_then(|index| self.questions.get(index).copied())
            .unwrap_or(false)
    }

    fn set_question(&mut self, row: usize, col: usize, value: bool) {
        let index = row
            .checked_mul(self.game.cols())
            .and_then(|value| value.checked_add(col));
        if let Some(index) = index {
            if let Some(slot) = self.questions.get_mut(index) {
                *slot = value;
            }
        }
    }

    fn reset_questions(&mut self) {
        let count = self.game.rows().saturating_mul(self.game.cols());
        self.questions = vec![false; count];
    }

    fn clear_round_state(&mut self) {
        self.timer_started = None;
        self.elapsed = 0;
        self.left_down = false;
        self.pressed = Pressed::None;
        self.exploded_cell = None;
        self.questions.fill(false);
    }

    fn reset_current(&mut self) {
        self.game.reset(next_seed());
        self.reset_questions();
        self.clear_round_state();
    }

    fn start_difficulty(&mut self, difficulty: Difficulty) {
        self.game.set_difficulty(difficulty, next_seed());
        self.reset_questions();
        self.clear_round_state();
    }

    fn begin_timer_if_playing(&mut self) {
        if self.game.status() == GameStatus::Playing && self.timer_started.is_none() {
            self.timer_started = Some(Instant::now());
        }
    }

    fn refresh_elapsed(&mut self) -> bool {
        let old = self.elapsed;
        if let Some(started) = self.timer_started {
            self.elapsed = started.elapsed().as_secs().min(999);
        }
        self.elapsed != old
    }

    fn settle_timer(&mut self) -> bool {
        let changed = self.refresh_elapsed();
        if self.is_terminal() {
            self.timer_started = None;
        }
        changed
    }

    fn timer_tick(&mut self) -> EventResult {
        if self.timer_started.is_none() {
            return EventResult::NONE;
        }
        if self.refresh_elapsed() {
            EventResult::REDRAW
        } else {
            EventResult::NONE
        }
    }

    fn finish_cell_action(&mut self, row: usize, col: usize, direct_reveal: bool) {
        self.begin_timer_if_playing();
        let status = self.game.status();
        if status == GameStatus::Lost {
            self.exploded_cell = if direct_reveal
                && self
                    .game
                    .cell(row, col)
                    .map(|cell| cell.mine)
                    .unwrap_or(false)
            {
                Some((row, col))
            } else {
                None
            };
        } else {
            self.exploded_cell = None;
        }
        if status == GameStatus::Won {
            self.questions.fill(false);
        }
        let _ = self.settle_timer();
    }

    fn activate_cell(&mut self, row: usize, col: usize) -> bool {
        if self.is_terminal() {
            return false;
        }
        let Some(cell) = self.game.cell(row, col) else {
            return false;
        };
        match cell.state {
            CellState::Hidden => {
                self.set_question(row, col, false);
                if self.game.reveal(row, col) {
                    self.finish_cell_action(row, col, true);
                    true
                } else {
                    false
                }
            }
            CellState::Revealed => {
                if self.game.chord(row, col) {
                    self.finish_cell_action(row, col, false);
                    true
                } else {
                    false
                }
            }
            CellState::Flagged => false,
        }
    }

    fn handle_right_click(&mut self, x: i32, y: i32) -> EventResult {
        if self.is_terminal() {
            return EventResult::NONE;
        }
        let layout = self.layout();
        let Some((row, col)) = layout.cell_at(Point::new(x, y)) else {
            return EventResult::NONE;
        };
        let Some(cell) = self.game.cell(row, col) else {
            return EventResult::NONE;
        };
        match cell.state {
            CellState::Flagged => {
                if self.game.toggle_flag(row, col) {
                    if self.marks_enabled {
                        self.set_question(row, col, true);
                    }
                    EventResult::REDRAW
                } else {
                    EventResult::NONE
                }
            }
            CellState::Hidden => {
                if self.question_at(row, col) {
                    self.set_question(row, col, false);
                    EventResult::REDRAW
                } else if self.game.toggle_flag(row, col) {
                    EventResult::REDRAW
                } else {
                    EventResult::NONE
                }
            }
            CellState::Revealed => EventResult::NONE,
        }
    }

    fn handle_middle_click(&mut self, x: i32, y: i32) -> EventResult {
        if self.is_terminal() {
            return EventResult::NONE;
        }
        let layout = self.layout();
        let Some((row, col)) = layout.cell_at(Point::new(x, y)) else {
            return EventResult::NONE;
        };
        if self.game.chord(row, col) {
            self.finish_cell_action(row, col, false);
            EventResult::REDRAW
        } else {
            EventResult::NONE
        }
    }

    fn press_target_at(&self, x: i32, y: i32) -> Pressed {
        let layout = self.layout();
        let point = Point::new(x, y);
        if layout.smiley.contains(point) {
            return Pressed::Smiley;
        }
        if self.is_terminal() {
            return Pressed::Elsewhere;
        }
        if let Some((row, col)) = layout.cell_at(point) {
            if let Some(cell) = self.game.cell(row, col) {
                if cell.state != CellState::Flagged {
                    return Pressed::Cell(row, col);
                }
            }
        }
        Pressed::Elsewhere
    }

    fn handle_left_down(&mut self, x: i32, y: i32) -> EventResult {
        self.left_down = true;
        self.pressed = self.press_target_at(x, y);
        EventResult::REDRAW
    }

    fn handle_left_up(&mut self, x: i32, y: i32) -> EventResult {
        let was_down = self.left_down;
        let previous = self.pressed;
        let current = self.press_target_at(x, y);
        self.left_down = false;
        self.pressed = Pressed::None;

        match (previous, current) {
            (Pressed::Smiley, Pressed::Smiley) => self.reset_current(),
            (Pressed::Cell(row, col), Pressed::Cell(current_row, current_col))
                if row == current_row && col == current_col =>
            {
                let _ = self.activate_cell(row, col);
            }
            _ => {}
        }

        if was_down {
            EventResult::REDRAW
        } else {
            EventResult::NONE
        }
    }

    fn handle_command(&mut self, command: u16) -> EventResult {
        match command {
            win32::CMD_NEW_GAME => {
                self.reset_current();
                EventResult::REDRAW
            }
            win32::CMD_BEGINNER => {
                self.start_difficulty(Difficulty::Beginner);
                EventResult::REDRAW
            }
            win32::CMD_INTERMEDIATE => {
                self.start_difficulty(Difficulty::Intermediate);
                EventResult::REDRAW
            }
            win32::CMD_EXPERT => {
                self.start_difficulty(Difficulty::Expert);
                EventResult::REDRAW
            }
            win32::CMD_MARKS => {
                self.marks_enabled = !self.marks_enabled;
                if !self.marks_enabled {
                    self.questions.fill(false);
                }
                EventResult::REDRAW
            }
            win32::CMD_EXIT => EventResult::QUIT,
            win32::CMD_ABOUT => {
                win32::show_about();
                EventResult::NONE
            }
            _ => EventResult::NONE,
        }
    }
}

impl WindowApp for App {
    fn paint(&mut self, canvas: &mut Canvas, width: i32, height: i32) {
        self.window_width = width;
        self.window_height = height;
        let layout = ClassicLayout::new(self.game.difficulty(), width, height);
        canvas.fill_rect(Rect::new(0, 0, width, height), BACKGROUND);
        draw_top_panel(canvas, self, layout);
        draw_minefield(canvas, self, layout);
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::MouseMove { x, y } => {
                if self.left_down {
                    let next = self.press_target_at(x, y);
                    if next != self.pressed {
                        self.pressed = next;
                        return EventResult::REDRAW;
                    }
                }
                EventResult::NONE
            }
            Event::MouseLeave => {
                if self.left_down && self.pressed != Pressed::Elsewhere {
                    self.pressed = Pressed::Elsewhere;
                    EventResult::REDRAW
                } else {
                    EventResult::NONE
                }
            }
            Event::LeftDown { x, y } => self.handle_left_down(x, y),
            Event::LeftUp { x, y } => self.handle_left_up(x, y),
            Event::RightDown { x, y } => self.handle_right_click(x, y),
            Event::MiddleDown { x, y } => self.handle_middle_click(x, y),
            Event::Timer => self.timer_tick(),
            Event::KeyDown(key) => match key {
                0x1B => EventResult::QUIT,
                0x72 | 0x52 => {
                    self.reset_current();
                    EventResult::REDRAW
                }
                _ => EventResult::NONE,
            },
            Event::Command(command) => self.handle_command(command),
            Event::FocusLost => {
                let changed = self.left_down || self.pressed != Pressed::None;
                self.left_down = false;
                self.pressed = Pressed::None;
                if changed {
                    EventResult::REDRAW
                } else {
                    EventResult::NONE
                }
            }
        }
    }

    fn preferred_client_size(&self) -> (i32, i32) {
        preferred_client_size(self.game.difficulty())
    }

    fn menu_state(&self) -> MenuState {
        MenuState {
            difficulty_index: self.menu_difficulty_index(),
            marks_enabled: self.marks_enabled,
        }
    }
}

pub fn run() -> Result<(), String> {
    let app = App::new()?;
    let (width, height) = preferred_client_size(app.game.difficulty());
    win32::run_window(Box::new(app), TITLE, width, height)
}

fn next_seed() -> u64 {
    let counter = SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_nanos() as u64;
    now ^ counter.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xA5A5_5A5A_0123_4567
}

fn fill_bevel_box(canvas: &mut Canvas, rect: Rect, thickness: i32, raised: bool) {
    let thickness = thickness.max(1);
    let top_left = if raised { WHITE } else { FACE_DARK };
    let bottom_right = if raised { FACE_DARK } else { WHITE };
    canvas.fill_rect(
        Rect::new(rect.left, rect.top, rect.right, rect.top + thickness),
        top_left,
    );
    canvas.fill_rect(
        Rect::new(rect.left, rect.top, rect.left + thickness, rect.bottom),
        top_left,
    );
    canvas.fill_rect(
        Rect::new(rect.left, rect.bottom - thickness, rect.right, rect.bottom),
        bottom_right,
    );
    canvas.fill_rect(
        Rect::new(rect.right - thickness, rect.top, rect.right, rect.bottom),
        bottom_right,
    );
}

fn draw_top_panel(canvas: &mut Canvas, app: &App, layout: ClassicLayout) {
    canvas.fill_rect(layout.panel, BACKGROUND);
    fill_bevel_box(canvas, layout.panel, 2, false);

    draw_led(
        canvas,
        layout.led_left,
        app.game.flags_remaining().clamp(0, 999),
    );
    draw_led(canvas, layout.led_right, app.elapsed.min(999) as i32);

    let pressed = app.left_down && app.pressed == Pressed::Smiley;
    draw_smiley_button(canvas, layout.smiley, app.face_state(), pressed);
}

fn draw_led(canvas: &mut Canvas, rect: Rect, value: i32) {
    canvas.fill_rect(rect, BLACK);
    fill_bevel_box(canvas, rect, 2, false);

    let total_width = DIGIT_WIDTH * 3 + DIGIT_GAP * 2;
    let start_x = rect.center_x() - total_width / 2;
    let y = rect.center_y() - DIGIT_HEIGHT / 2;
    let value = value.clamp(0, 999) as u32;
    let digits = [
        (value / 100 % 10) as u8,
        (value / 10 % 10) as u8,
        (value % 10) as u8,
    ];
    for (index, digit) in digits.iter().enumerate() {
        let x = start_x + index as i32 * (DIGIT_WIDTH + DIGIT_GAP);
        draw_seven_segment_digit(canvas, x, y, *digit);
    }
}

fn seven_segment_mask(digit: u8) -> u8 {
    match digit {
        0 => 0b011_1111,
        1 => 0b000_0110,
        2 => 0b101_1011,
        3 => 0b100_1111,
        4 => 0b110_0110,
        5 => 0b110_1101,
        6 => 0b111_1101,
        7 => 0b000_0111,
        8 => 0b111_1111,
        9 => 0b110_1111,
        _ => 0,
    }
}

fn draw_seven_segment_digit(canvas: &mut Canvas, x: i32, y: i32, digit: u8) {
    let mask = seven_segment_mask(digit);
    let half = DIGIT_HEIGHT / 2;
    let middle_y = y + half - SEGMENT / 2;
    let segments = [
        (
            0b000_0001,
            Rect::from_xywh(x + SEGMENT, y, DIGIT_WIDTH - SEGMENT * 2, SEGMENT),
        ),
        (
            0b000_0010,
            Rect::from_xywh(
                x + DIGIT_WIDTH - SEGMENT,
                y + SEGMENT,
                SEGMENT,
                half - SEGMENT,
            ),
        ),
        (
            0b000_0100,
            Rect::from_xywh(x + DIGIT_WIDTH - SEGMENT, y + half, SEGMENT, half - SEGMENT),
        ),
        (
            0b000_1000,
            Rect::from_xywh(
                x + SEGMENT,
                y + DIGIT_HEIGHT - SEGMENT,
                DIGIT_WIDTH - SEGMENT * 2,
                SEGMENT,
            ),
        ),
        (
            0b001_0000,
            Rect::from_xywh(x, y + half, SEGMENT, half - SEGMENT),
        ),
        (
            0b010_0000,
            Rect::from_xywh(x, y + SEGMENT, SEGMENT, half - SEGMENT),
        ),
        (
            0b100_0000,
            Rect::from_xywh(x + SEGMENT, middle_y, DIGIT_WIDTH - SEGMENT * 2, SEGMENT),
        ),
    ];

    for (bit, segment) in segments {
        let color = if mask & bit != 0 { LED_ON } else { LED_OFF };
        canvas.fill_rect(segment, color);
    }
}

fn draw_smiley_button(canvas: &mut Canvas, rect: Rect, state: FaceState, pressed: bool) {
    canvas.fill_rect(rect, BACKGROUND);
    fill_bevel_box(canvas, rect, 2, !pressed);

    let face_size = 28;
    let face = Rect::from_xywh(
        rect.center_x() - face_size / 2,
        rect.center_y() - face_size / 2,
        face_size,
        face_size,
    );
    canvas.ellipse(face, YELLOW, BLACK, 1);

    let cx = face.center_x();
    let cy = face.center_y();
    match state {
        FaceState::Normal => {
            canvas.fill_rect(Rect::from_xywh(cx - 7, cy - 6, 3, 3), BLACK);
            canvas.fill_rect(Rect::from_xywh(cx + 4, cy - 6, 3, 3), BLACK);
            draw_smile(canvas, cx, cy, false);
        }
        FaceState::Surprised => {
            canvas.fill_rect(Rect::from_xywh(cx - 8, cy - 7, 4, 4), BLACK);
            canvas.fill_rect(Rect::from_xywh(cx + 4, cy - 7, 4, 4), BLACK);
            canvas.ellipse(Rect::from_xywh(cx - 3, cy + 3, 7, 8), BLACK, BLACK, 1);
        }
        FaceState::Won => {
            canvas.fill_rect(Rect::from_xywh(cx - 10, cy - 7, 8, 5), BLACK);
            canvas.fill_rect(Rect::from_xywh(cx + 2, cy - 7, 8, 5), BLACK);
            canvas.fill_rect(Rect::from_xywh(cx - 2, cy - 6, 4, 2), BLACK);
            draw_smile(canvas, cx, cy, true);
        }
        FaceState::Lost => {
            draw_pixel_x(canvas, cx - 6, cy - 5, 3);
            draw_pixel_x(canvas, cx + 6, cy - 5, 3);
            let frown = [
                Point::new(cx - 7, cy + 8),
                Point::new(cx - 4, cy + 4),
                Point::new(cx, cy + 2),
                Point::new(cx + 4, cy + 4),
                Point::new(cx + 7, cy + 8),
            ];
            canvas.polyline(&frown, BLACK, 2);
        }
    }
}

fn draw_smile(canvas: &mut Canvas, cx: i32, cy: i32, wide: bool) {
    let radius = if wide { 8 } else { 7 };
    let points = [
        Point::new(cx - radius, cy + 2),
        Point::new(cx - 4, cy + 6),
        Point::new(cx, cy + 8),
        Point::new(cx + 4, cy + 6),
        Point::new(cx + radius, cy + 2),
    ];
    canvas.polyline(&points, BLACK, 2);
}

fn draw_pixel_x(canvas: &mut Canvas, cx: i32, cy: i32, radius: i32) {
    canvas.line(cx - radius, cy - radius, cx + radius, cy + radius, BLACK, 2);
    canvas.line(cx + radius, cy - radius, cx - radius, cy + radius, BLACK, 2);
}

fn draw_minefield(canvas: &mut Canvas, app: &App, layout: ClassicLayout) {
    canvas.fill_rect(layout.board, BACKGROUND);
    fill_bevel_box(canvas, layout.board, 3, false);
    canvas.fill_rect(layout.grid, BACKGROUND);

    let status = app.game.status();
    for row in 0..layout.rows {
        for col in 0..layout.cols {
            if let Some(cell) = app.game.cell(row, col) {
                draw_cell(canvas, app, layout, row, col, cell, status);
            }
        }
    }
}

fn draw_cell(
    canvas: &mut Canvas,
    app: &App,
    layout: ClassicLayout,
    row: usize,
    col: usize,
    cell: CellView,
    status: GameStatus,
) {
    let rect = layout.cell_rect(row, col);
    let pressed = app.left_down && app.pressed == Pressed::Cell(row, col) && !app.is_terminal();

    if status == GameStatus::Lost && cell.mine && cell.state != CellState::Flagged {
        let exploded = app.exploded_cell == Some((row, col));
        draw_revealed_cell(canvas, rect, if exploded { RED } else { BACKGROUND });
        draw_mine(canvas, rect);
        return;
    }

    match cell.state {
        CellState::Flagged => {
            draw_hidden_cell(canvas, rect, false);
            draw_flag(canvas, rect);
            if status == GameStatus::Lost && !cell.mine {
                draw_wrong_flag_cross(canvas, rect);
            }
        }
        CellState::Revealed => {
            draw_revealed_cell(canvas, rect, BACKGROUND);
            if cell.adjacent > 0 {
                draw_cell_number(canvas, rect, cell.adjacent, &app.fonts.number);
            }
        }
        CellState::Hidden => {
            draw_hidden_cell(canvas, rect, pressed);
            if app.question_at(row, col) {
                draw_question_mark(canvas, rect);
            }
        }
    }
}

fn draw_hidden_cell(canvas: &mut Canvas, rect: Rect, pressed: bool) {
    canvas.fill_rect(rect, BACKGROUND);
    fill_bevel_box(canvas, rect, 3, !pressed);
}

fn draw_revealed_cell(canvas: &mut Canvas, rect: Rect, fill: Color) {
    canvas.fill_rect(rect, fill);
    canvas.fill_rect(
        Rect::new(rect.left, rect.top, rect.right, rect.top + 1),
        FACE_DARK,
    );
    canvas.fill_rect(
        Rect::new(rect.left, rect.top, rect.left + 1, rect.bottom),
        FACE_DARK,
    );
    canvas.fill_rect(
        Rect::new(rect.left, rect.bottom - 1, rect.right, rect.bottom),
        WHITE,
    );
    canvas.fill_rect(
        Rect::new(rect.right - 1, rect.top, rect.right, rect.bottom),
        WHITE,
    );
}

fn draw_cell_number(canvas: &mut Canvas, rect: Rect, number: u8, font: &Font) {
    let (text, color) = match number {
        1 => ("1", NUMBER_1),
        2 => ("2", NUMBER_2),
        3 => ("3", NUMBER_3),
        4 => ("4", NUMBER_4),
        5 => ("5", NUMBER_5),
        6 => ("6", NUMBER_6),
        7 => ("7", NUMBER_7),
        8 => ("8", NUMBER_8),
        _ => return,
    };
    canvas.text(
        text,
        Rect::new(rect.left, rect.top - 1, rect.right, rect.bottom + 1),
        font,
        color,
        DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX,
    );
}

fn draw_question_mark(canvas: &mut Canvas, rect: Rect) {
    let cx = rect.center_x();
    let top = rect.top + 7;
    canvas.fill_rect(Rect::from_xywh(cx - 5, top, 10, 3), BLACK);
    canvas.fill_rect(Rect::from_xywh(cx + 4, top + 2, 3, 6), BLACK);
    canvas.fill_rect(Rect::from_xywh(cx - 1, top + 6, 5, 3), BLACK);
    canvas.fill_rect(Rect::from_xywh(cx - 1, top + 8, 3, 6), BLACK);
    canvas.fill_rect(Rect::from_xywh(cx - 1, top + 17, 3, 3), BLACK);
}

fn draw_flag(canvas: &mut Canvas, rect: Rect) {
    let cx = rect.center_x();
    let pole_x = cx - 2;
    let top = rect.top + 5;
    let bottom = rect.bottom - 6;
    canvas.line(pole_x, top, pole_x, bottom, BLACK, 2);
    canvas.line(pole_x - 5, bottom, pole_x + 3, bottom, BLACK, 3);
    let pennant = [
        Point::new(pole_x, top + 1),
        Point::new(pole_x + 10, top + 5),
        Point::new(pole_x, top + 11),
    ];
    canvas.polygon(&pennant, RED, DARK_RED, 1);
    canvas.line(pole_x + 1, top + 2, pole_x + 1, top + 7, RED, 1);
}

fn draw_wrong_flag_cross(canvas: &mut Canvas, rect: Rect) {
    let inset = 4;
    canvas.line(
        rect.left + inset,
        rect.top + inset,
        rect.right - inset,
        rect.bottom - inset,
        DARK_RED,
        3,
    );
    canvas.line(
        rect.left + inset,
        rect.top + inset,
        rect.right - inset,
        rect.bottom - inset,
        RED,
        1,
    );
    canvas.line(
        rect.right - inset,
        rect.top + inset,
        rect.left + inset,
        rect.bottom - inset,
        DARK_RED,
        3,
    );
    canvas.line(
        rect.right - inset,
        rect.top + inset,
        rect.left + inset,
        rect.bottom - inset,
        RED,
        1,
    );
}

fn draw_mine(canvas: &mut Canvas, rect: Rect) {
    let cx = rect.center_x();
    let cy = rect.center_y();
    let radius = 7;
    let spike = 11;
    let diagonal = 8;
    let ends = [
        (0, -spike),
        (spike, 0),
        (0, spike),
        (-spike, 0),
        (diagonal, -diagonal),
        (diagonal, diagonal),
        (-diagonal, diagonal),
        (-diagonal, -diagonal),
    ];
    for (dx, dy) in ends {
        canvas.line(cx, cy, cx + dx, cy + dy, BLACK, 2);
    }
    canvas.ellipse(
        Rect::from_xywh(cx - radius, cy - radius, radius * 2, radius * 2),
        BLACK,
        BLACK,
        1,
    );
    canvas.fill_rect(Rect::from_xywh(cx - 3, cy - 3, 3, 3), WHITE);
    canvas.fill_rect(Rect::from_xywh(cx - 1, cy - 1, 2, 2), WHITE);
}
