use crossterm::{
    cursor::{Hide, Show},
    event::{poll, read, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind, EnableMouseCapture, DisableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::Rng;
use std::{io::stdout, time::Duration};

// Removed fixed dimensions; we'll use the full terminal size dynamically.

const SAND_CHAR: char = '▪'; // Changed to a smaller block character
const EMPTY_CHAR: char = ' ';
const SPAWN_RATE: usize = 3; // Number of sand particles to spawn per frame

struct World {
    grid: Vec<char>,
    width: usize,
    height: usize,
}

impl World {
    fn new(width: usize, height: usize) -> Self {
        World {
            grid: vec![EMPTY_CHAR; width * height],
            width,
            height,
        }
    }

    fn update(&mut self) {
        let mut rng = rand::thread_rng();
        for y in (1..self.height).rev() {
            for x in 0..self.width {
                let src = (y - 1) * self.width + x; // cell above
                if self.grid[src] == SAND_CHAR {
                    let dst = y * self.width + x; // current cell
                    if self.grid[dst] == EMPTY_CHAR {
                        // Fall straight down
                        self.grid[dst] = SAND_CHAR;
                        self.grid[src] = EMPTY_CHAR;
                    } else {
                        let left_free = x > 0 && self.grid[y * self.width + (x - 1)] == EMPTY_CHAR;
                        let right_free = x < self.width - 1 && self.grid[y * self.width + (x + 1)] == EMPTY_CHAR;

                        if left_free && right_free {
                            // Randomly choose to slide left or right
                            if rng.gen_bool(0.5) {
                                self.grid[y * self.width + (x - 1)] = SAND_CHAR;
                                self.grid[src] = EMPTY_CHAR;
                            } else {
                                self.grid[y * self.width + (x + 1)] = SAND_CHAR;
                                self.grid[src] = EMPTY_CHAR;
                            }
                        } else if left_free {
                            self.grid[y * self.width + (x - 1)] = SAND_CHAR;
                            self.grid[src] = EMPTY_CHAR;
                        } else if right_free {
                            self.grid[y * self.width + (x + 1)] = SAND_CHAR;
                            self.grid[src] = EMPTY_CHAR;
                        }
                    }
                }
            }
        }
    }

    fn spawn_sand(&mut self) {
        let mut rng = rand::thread_rng();
        // Spawn multiple sand particles per frame at the top row
        for _ in 0..SPAWN_RATE {
            let x = rng.gen_range(0..self.width);
            // cell (0, x) is at index x
            if self.grid[x] == EMPTY_CHAR {
                self.grid[x] = SAND_CHAR;
            }
        }
    }

    fn render(&mut self) {
        // Clear screen and move cursor to top-left
        print!("\x1B[2J\x1B[1;1H");
        
        // Add an ANSI sequence to reduce line spacing
        print!("\x1B[0;1m\x1B[0m"); // Reset all attributes
        print!("\x1B[?7l"); // Disable line wrapping
        
        for y in 0..self.height {
            for x in 0..self.width {
                print!("{}", self.grid[y * self.width + x]);
            }
            // Use a carriage return with line feed to minimize line spacing
            print!("\r\n");
        }
        
        // Re-enable line wrapping
        print!("\x1B[?7h");
    }
}

// Add this helper function to interpolate between two points using Bresenham's algorithm.
fn draw_line(grid: &mut Vec<char>, width: usize, height: usize, x0: usize, y0: usize, x1: usize, y1: usize) {
    // Convert to isize for calculations.
    let dx = if x1 > x0 { x1 - x0 } else { x0 - x1 } as isize;
    let dy = if y1 > y0 { y1 - y0 } else { y0 - y1 } as isize;
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = if dx > dy { dx } else { -dy } / 2;
    let mut x = x0 as isize;
    let mut y = y0 as isize;
    let x1 = x1 as isize;
    let y1 = y1 as isize;
    loop {
        // Only write if within bounds.
        if x >= 0 && (x as usize) < width && y >= 0 && (y as usize) < height {
            grid[y as usize * width + x as usize] = SAND_CHAR;
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = err;
        if e2 > -dx {
            err -= dy;
            x += sx;
        }
        if e2 < dy {
            err += dx;
            y += sy;
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up terminal with mouse capture enabled
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide, EnableMouseCapture)?;

    // Dynamically determine the full terminal size
    let (cols, rows) = crossterm::terminal::size()?;
    let width = cols as usize;
    let height = rows as usize;

    let mut world = World::new(width, height);
    
    // --- Added state tracking for continuous mouse drawing ---
    let mut mouse_down = false;
    let mut mouse_x = 0;
    let mut mouse_y = 0;
    // Added previous mouse coordinates for line interpolation
    let mut prev_mouse_x = 0;
    let mut prev_mouse_y = 0;
    // ----------------------------------------------------------

    'main_loop: loop {
        // Process all available input events (non-blocking)
        while poll(Duration::from_millis(0))? {
            match read()? {
                Event::Key(key_event) => {
                    if key_event.code == KeyCode::Esc ||
                       (key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL))
                    {
                        break 'main_loop;
                    }
                },
                Event::Mouse(mouse_event) => {
                    // --- Updated mouse event handler for continuous spawning with interpolation ---
                    match mouse_event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            mouse_down = true;
                            mouse_x = mouse_event.column as usize;
                            mouse_y = mouse_event.row as usize;
                            // Initialize previous coordinates on mouse down.
                            prev_mouse_x = mouse_x;
                            prev_mouse_y = mouse_y;
                        },
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let new_x = mouse_event.column as usize;
                            let new_y = mouse_event.row as usize;
                            // Interpolate between the previous and new points.
                            draw_line(&mut world.grid, world.width, world.height, prev_mouse_x, prev_mouse_y, new_x, new_y);
                            mouse_x = new_x;
                            mouse_y = new_y;
                            prev_mouse_x = new_x;
                            prev_mouse_y = new_y;
                        },
                        MouseEventKind::Up(MouseButton::Left) => {
                            mouse_down = false;
                        },
                        _ => {}
                    }
                    // -------------------------------------------------------------------------
                },
                _ => {}
            }
        }

        // In case there was no drag event this frame, still color the current mouse cell.
        if mouse_down && mouse_x < world.width && mouse_y < world.height {
            world.grid[mouse_y * world.width + mouse_x] = SAND_CHAR;
        }

        world.update();
        world.render();
        std::thread::sleep(Duration::from_millis(50));
    }

    // Cleanup terminal by turning off mouse capture as well.
    execute!(stdout(), Show, LeaveAlternateScreen, DisableMouseCapture)?;
    disable_raw_mode()?;
    Ok(())
} 