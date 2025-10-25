use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::{Rng, rngs::ThreadRng};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::{
    io,
    time::{Duration, Instant},
};

/// Represents a position (x, y) on the board
#[derive(Clone, Copy, PartialEq, Eq)]
struct Point {
    x: u16,
    y: u16,
}

/// Snake movement directions
#[derive(Clone, Copy, PartialEq)]
enum DirectionEnum {
    Up,
    Down,
    Left,
    Right,
}

/// Main game state
struct Game {
    snake: Vec<Point>,
    dir: DirectionEnum,
    next_dir: DirectionEnum,
    apple: Point,
    rng: ThreadRng,
    score: u32,
    width: u16,
    height: u16,
    game_over: bool,
    level: u32,
    base_tick_ms: u64,
}

impl Game {
    /// Initializes a new game session
    fn new(area: Rect) -> Self {
        let width = area.width.saturating_sub(2).max(10);
        let height = area.height.saturating_sub(4).max(5);
        let rng = rand::thread_rng();

        let mid_x = width / 2;
        let mid_y = height / 2;
        let snake = vec![
            Point { x: mid_x, y: mid_y },
            Point {
                x: mid_x.saturating_sub(1),
                y: mid_y,
            },
            Point {
                x: mid_x.saturating_sub(2),
                y: mid_y,
            },
        ];

        let mut g = Self {
            snake,
            dir: DirectionEnum::Right,
            next_dir: DirectionEnum::Right,
            apple: Point { x: 0, y: 0 },
            rng,
            score: 0,
            width,
            height,
            game_over: false,
            level: 1,
            base_tick_ms: 160,
        };
        g.place_apple();
        g
    }

    /// Places a new apple randomly on the board
    fn place_apple(&mut self) {
        for _ in 0..1000 {
            let x = self.rng.gen_range(0..self.width);
            let y = self.rng.gen_range(0..self.height);
            let cand = Point { x, y };
            if !self.snake.iter().any(|s| s.x == x && s.y == y) {
                self.apple = cand;
                return;
            }
        }
        self.apple = Point { x: 1, y: 1 };
    }

    /// Changes snake direction (no reverse allowed)
    fn set_direction(&mut self, d: DirectionEnum) {
        let is_reverse = matches!(
            (self.dir, d),
            (DirectionEnum::Up, DirectionEnum::Down)
                | (DirectionEnum::Down, DirectionEnum::Up)
                | (DirectionEnum::Left, DirectionEnum::Right)
                | (DirectionEnum::Right, DirectionEnum::Left)
        );
        if !is_reverse {
            self.next_dir = d;
        }
    }

    /// Game tick — moves snake, checks collisions, updates score
    fn step(&mut self) {
        if self.game_over {
            return;
        }
        self.dir = self.next_dir;
        let head = self.snake[0];
        let new_head = match self.dir {
            DirectionEnum::Up => Point {
                x: head.x,
                y: head.y.saturating_sub(1),
            },
            DirectionEnum::Down => Point {
                x: head.x,
                y: head.y.saturating_add(1),
            },
            DirectionEnum::Left => Point {
                x: head.x.saturating_sub(1),
                y: head.y,
            },
            DirectionEnum::Right => Point {
                x: head.x.saturating_add(1),
                y: head.y,
            },
        };

        // Check collisions with borders or itself
        if new_head.x >= self.width || new_head.y >= self.height {
            self.game_over = true;
            return;
        }
        if self
            .snake
            .iter()
            .any(|s| s.x == new_head.x && s.y == new_head.y)
        {
            self.game_over = true;
            return;
        }

        // Move snake forward
        self.snake.insert(0, new_head);

        // Check apple collision
        if new_head.x == self.apple.x && new_head.y == self.apple.y {
            self.score += 1;
            if self.score % 5 == 0 {
                self.level = 1 + (self.score / 5);
            }
            self.place_apple();
        } else {
            self.snake.pop();
        }
    }

    /// Controls snake speed (faster with higher levels)
    fn tick_duration(&self) -> Duration {
        let reduce = (self.level - 1) as u64 * 10;
        let ms = self.base_tick_ms.saturating_sub(reduce).max(40);
        Duration::from_millis(ms)
    }
}

/// Draws the main game screen
fn draw_game<B: ratatui::backend::Backend>(f: &mut Frame<B>, game: &Game, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(2),
            ]
            .as_ref(),
        )
        .split(area);

    // Header with score and level
    let title = Paragraph::new(Line::from(vec![
        Span::styled(
            " Snake (Rust + ratatui) ",
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Score: {}", game.score),
            Style::default().fg(Color::LightGreen),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Level: {}", game.level),
            Style::default().fg(Color::Cyan),
        ),
    ]))
    .alignment(Alignment::Left);
    f.render_widget(title, chunks[0]);

    // Game board area
    let board_block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Game ", Style::default().fg(Color::Magenta)));
    let inner = board_block.inner(chunks[1]);
    f.render_widget(board_block, chunks[1]);

    // Render snake and apple
    let mut rows: Vec<Line> = Vec::new();
    for y in 0..game.height {
        let mut spans = Vec::new();
        for x in 0..game.width {
            let (ch, style) = if x == game.apple.x && y == game.apple.y {
                (
                    "@",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )
            } else if let Some((i, _)) = game
                .snake
                .iter()
                .enumerate()
                .find(|(_, p)| p.x == x && p.y == y)
            {
                if i == 0 {
                    (
                        "■",
                        Style::default()
                            .fg(Color::LightGreen)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    ("■", Style::default().fg(Color::Green))
                }
            } else {
                (" ", Style::default().bg(Color::Black))
            };
            spans.push(Span::styled(ch, style));
        }
        rows.push(Line::from(spans));
    }

    let board = Paragraph::new(rows).alignment(Alignment::Left);
    f.render_widget(board, inner);

    // Bottom info line with controls
    let mut status_text = vec![
        Span::raw("Use "),
        Span::styled("W A S D", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to move. "),
        Span::styled("Q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(" to quit."),
    ];

    // Show restart prompt on game over
    if game.game_over {
        status_text.push(Span::raw("  "));
        status_text.push(Span::styled(
            "GAME OVER - Press R to restart or Q to quit",
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
    }

    let status = Paragraph::new(Line::from(status_text)).alignment(Alignment::Left);
    f.render_widget(status, chunks[2]);
}

/// Draws the main menu screen
fn draw_menu<B: ratatui::backend::Backend>(f: &mut Frame<B>, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title("Snake - Menu");
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };
    let lines = vec![
        Line::from(Span::styled(
            "Welcome to Snake (Terminal Edition)",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(" ")),
        Line::from(Span::raw("Press Enter to start")),
        Line::from(Span::raw("Press Q to quit")),
    ];
    let p = Paragraph::new(lines).alignment(Alignment::Center);
    f.render_widget(p, inner);
}

/// Entry point
fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let res = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error: {:?}", err);
    }
    Ok(())
}

/// Game loop: handles menu, game, and restart logic
fn run_app<B: ratatui::backend::Backend>(terminal: &mut Terminal<B>) -> io::Result<()> {
    let mut show_menu = true;
    let mut game_opt: Option<Game> = None;

    loop {
        // Draw either the menu or the game
        terminal.draw(|f| {
            let size = f.size();
            if show_menu {
                draw_menu(f, size);
            } else if let Some(g) = &game_opt {
                draw_game(f, g, size);
            }
        })?;

        // Menu input handling
        if show_menu {
            if event::poll(Duration::from_millis(200))? {
                if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                    match code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                        KeyCode::Enter => {
                            let size = terminal.get_frame().size();
                            game_opt = Some(Game::new(size));
                            show_menu = false;
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }

        // Main game loop
        if let Some(game) = game_opt.as_mut() {
            let tick_dur = game.tick_duration();
            let mut last_tick = Instant::now();

            loop {
                terminal.draw(|f| {
                    draw_game(f, game, f.size());
                })?;

                let timeout = Duration::from_millis(16);
                if event::poll(timeout)? {
                    match event::read()? {
                        // Quit game
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('q'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Char('Q'),
                            ..
                        }) => return Ok(()),
                        // Restart game instantly
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('r'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Char('R'),
                            ..
                        }) => {
                            let size = terminal.get_frame().size();
                            *game = Game::new(size);
                            break;
                        }
                        // Movement keys
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('w'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Up, ..
                        }) => game.set_direction(DirectionEnum::Up),
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('s'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Down,
                            ..
                        }) => game.set_direction(DirectionEnum::Down),
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('a'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Left,
                            ..
                        }) => game.set_direction(DirectionEnum::Left),
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('d'),
                            ..
                        })
                        | Event::Key(KeyEvent {
                            code: KeyCode::Right,
                            ..
                        }) => game.set_direction(DirectionEnum::Right),
                        _ => {}
                    }
                }

                // Update game state every tick
                if last_tick.elapsed() >= tick_dur {
                    game.step();
                    last_tick = Instant::now();
                }

                // Exit inner loop on Game Over
                if game.game_over {
                    break;
                }
            }

            // Game over loop: wait for R or Q
            loop {
                terminal.draw(|f| draw_game(f, game, f.size()))?;
                if event::poll(Duration::from_millis(200))? {
                    if let Event::Key(KeyEvent { code, .. }) = event::read()? {
                        match code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                            KeyCode::Char('r') | KeyCode::Char('R') => {
                                let size = terminal.get_frame().size();
                                *game = Game::new(size);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}
