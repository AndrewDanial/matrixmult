/// A simple example demonstrating how to handle user input. This is
/// a bit out of the scope of the library as it does not provide any
/// input handling out of the box. However, it may helps some to get
/// started.
///
/// This is a very simple example:
///   * A input box always focused. Every character you type is registered
///   here
///   * Pressing Backspace erases a character
///   * Pressing Enter pushes the current input in the history of previous
///   messages
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    error::Error,
    io,
    sync::{
        mpsc::{self, Receiver},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::Span,
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
    Frame, Terminal,
};

type Matrix = Vec<Vec<i64>>;

// the name event was taken :(
enum Ev<I> {
    Input(I),
    Tick,
}

/// App holds the state of the application
struct App {
    /// Selected Matrix
    curr_matrix: i32,
    /// The text inside either matrix
    matrix_text: Vec<String>,
    curr_string: String,
    answer: Option<Matrix>,
}

impl Default for App {
    fn default() -> App {
        App {
            curr_matrix: 0,
            matrix_text: vec![String::from(""); 2],
            curr_string: String::from(""),
            answer: None,
        }
    }
}

impl App {
    fn next(&mut self) {
        self.curr_string = String::from("");
        self.curr_matrix = (self.curr_matrix + 1) % 2;
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx) = mpsc::channel(); // create mpsc channel to handle inputs in separate thread
    let tick_rate = Duration::from_millis(1000); // wait 1000 ms for event
    thread::spawn(move || {
        let mut last_tick = Instant::now();
        loop {
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0)); // set timeout to tick_rate - (time since last_tick)

            // if we got an event
            if event::poll(timeout).unwrap() {
                // if the event is a keypress
                if let Event::Key(key) = event::read().unwrap() {
                    tx.send(Ev::Input(key)).unwrap();
                }
            }

            // if more than tick_rate time has passed since last_tick was created
            if last_tick.elapsed() >= tick_rate {
                if let Ok(_) = tx.send(Ev::Tick) {
                    last_tick = Instant::now(); // reset last tick
                }
            }
        }
    });

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    terminal.hide_cursor()?;
    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app, rx);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    rx: Receiver<Ev<KeyEvent>>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        match rx.recv().unwrap() {
            Ev::Input(key) => match key.code {
                KeyCode::Tab => {
                    app.next();
                }
                KeyCode::Char('q') => {
                    return Ok(());
                }
                KeyCode::Char(c) => match c {
                    '0'..='9' => {
                        app.matrix_text[app.curr_matrix as usize].push(c);
                        app.curr_string.push(c);
                    }
                    ' ' => {
                        app.matrix_text[app.curr_matrix as usize].push('_');
                        app.curr_string.push('_');
                    }
                    't' => {
                        parse_matrices(&mut app);
                    }
                    _ => {}
                },
                KeyCode::Enter => {
                    app.matrix_text[app.curr_matrix as usize].push('\n');
                    app.curr_string = String::from("");
                }

                KeyCode::Backspace => {
                    app.matrix_text[app.curr_matrix as usize].pop();
                    app.curr_string.pop();
                }
                _ => {}
            },
            Ev::Tick => {}
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(3)
        .constraints(
            [
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(10),
            ]
            .as_ref(),
        )
        .split(f.size());

    let matrices = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage(33),
                Constraint::Percentage(33),
                Constraint::Percentage(33),
            ]
            .as_ref(),
        )
        .split(chunks[2]);

    let text = |i: usize| {
        Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(25)])
            .split(matrices[i as usize])
    };

    for i in 0..3 {
        let para = render_grid(i, app.curr_matrix);
        f.render_widget(para, matrices[i as usize]);
    }

    for i in 0..app.matrix_text.len() {
        let a = Paragraph::new(app.matrix_text[i].as_ref())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(a, text(i)[1]);
    }

    if let Some(x) = &app.answer {
        let text2: Vec<Vec<String>> = x
            .iter()
            .map(|a| a.iter().map(|b| b.to_string()).collect())
            .collect();

        let mut str: String = String::from("");
        for i in text2 {
            for j in i {
                str.push_str(format!("{} ", j).as_str());
            }
            str.push_str("\n");
        }

        let a = Paragraph::new(str)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false });
        f.render_widget(a, text(2)[1]);
    }

    // let x = matrices[app.curr_matrix as usize].x;
    // let half_width = matrices[app.curr_matrix as usize].width / 2;
    // let len = app.curr_string.len() as u16;
    // let mut y_cal = matrices[app.curr_matrix as usize].y
    //     + matrices[app.curr_matrix as usize].height / 2
    //     + app.matrix_text[app.curr_matrix as usize]
    //         .split("\n")
    //         .collect::<Vec<_>>()
    //         .len() as u16
    //     - 3;
    // f.set_cursor(
    //     if x + len < 2 * half_width + 2 {
    //         x + len + 1
    //     } else {
    //         4
    //     },
    //     if x + len > 2 * half_width + 1 {
    //         y_cal += 1;
    //         y_cal
    //     } else {
    //         y_cal
    //     },
    // );
}

fn render_grid<'a>(index: i32, curr_matrix: i32) -> Paragraph<'a> {
    Paragraph::new("")
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style({
                    if curr_matrix == index {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default().fg(Color::White)
                    }
                })
                .border_type(BorderType::Plain)
                .title(Span::raw(if index != 2 {
                    format!("Matrix {}", index)
                } else {
                    String::from("Result")
                })),
        )
}

fn multiply_matrices(m1: &Matrix, m2: &Matrix) -> Matrix {
    let mut result = vec![vec![0; m2[0].len()]; m1.len()];

    for i in 0..m1.len() {
        // rows of the first matrix
        for j in 0..m2[0].len() {
            // columns of the second matrix
            for k in 0..m2.len() {
                // rows of the second matrix
                result[i][j] += m1[i][k] * m2[k][j]
            }
        }
    }

    result
}

fn multiply_matrices_threaded(m1: &Matrix, m2: &Matrix, thread_count: usize) -> Matrix {
    let mut threads = vec![];
    let (tx, rx) = mpsc::channel();

    let m1 = Arc::new(m1.clone());
    let m2 = Arc::new(m2.clone());

    for th in 0..thread_count {
        let tx = tx.clone();
        let m1 = Arc::new(m1.clone());
        let m2 = Arc::new(m2.clone());
        threads.push(thread::spawn(move || {
            println!("thread {} started", th);

            let mut curr_result = vec![vec![]; m1.len()];
            let start_row = (th * m1.len()) / thread_count;
            let end_row = ((th + 1) * m1.len()) / thread_count;
            // rows of the first matrix
            if start_row == end_row {
                return;
            }
            for i in start_row..end_row {
                // columns of the second matrix
                for j in 0..m2[0].len() {
                    // rows of the second matrix
                    let mut cur = 0;
                    for k in 0..m2.len() {
                        cur += m1[i][k] * m2[k][j]
                    }
                    curr_result[i].push(cur);
                }
            }
            tx.send((start_row, end_row, curr_result)).unwrap();
        }));
    }

    for i in threads {
        i.join().unwrap();
    }

    let mut result = vec![vec![]; m1.len()];
    for j in rx.iter().take(thread_count / 2) {
        let (start, end, m) = j;
        for i in start..end {
            result[i].extend(&m[i]);
        }
    }

    result
}

fn parse_matrices(app: &mut App) {
    let mut a = app.matrix_text[0].split("\n").collect::<Vec<_>>();
    let mut m1 = vec![vec![]; a.len()];
    for i in 0..a.len() {
        m1[i] = a[i].split("_").collect::<Vec<&str>>();
    }

    a = app.matrix_text[1].split("\n").collect::<Vec<_>>();
    let mut m2 = vec![vec![]; a.len()];
    for i in 0..a.len() {
        m2[i] = a[i].split("_").collect::<Vec<&str>>();
    }

    let m1: Matrix = m1
        .iter()
        .map(|a| a.iter().map(|b| b.parse::<i64>().unwrap()).collect())
        .collect();

    let m2: Matrix = m2
        .iter()
        .map(|a| a.iter().map(|b| b.parse::<i64>().unwrap()).collect())
        .collect();

    app.answer = Some(multiply_matrices(&m1, &m2));
}
