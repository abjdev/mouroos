use super::super::color::Color;
use super::super::framebuffer::Framebuffer;
use super::super::window::Application;
use alloc::format;
use alloc::vec::Vec;
use pc_keyboard::{DecodedKey, KeyCode};

const GRID_WIDTH: i32 = 22;
const GRID_HEIGHT: i32 = 18;
const CELL_SIZE: usize = 14;

#[derive(PartialEq, Eq)]
enum GameState {
    Playing,
    GameOver,
}

pub struct SnakeApp {
    snake: Vec<(i32, i32)>,
    dir: (i32, i32),
    next_dir: (i32, i32),
    food: (i32, i32),
    score: usize,
    high_score: usize,
    state: GameState,
    tick_timer: usize,
}

impl SnakeApp {
    pub fn new() -> Self {
        let mut app = SnakeApp {
            snake: Vec::new(),
            dir: (1, 0),
            next_dir: (1, 0),
            food: (12, 9),
            score: 0,
            high_score: 0,
            state: GameState::Playing,
            tick_timer: 0,
        };
        app.reset();
        app
    }

    fn reset(&mut self) {
        self.snake.clear();
        self.snake.push((6, 9));
        self.snake.push((5, 9));
        self.snake.push((4, 9));
        self.dir = (1, 0);
        self.next_dir = (1, 0);
        self.food = (14, 9);
        self.score = 0;
        self.state = GameState::Playing;
    }

    fn step(&mut self) {
        if self.state != GameState::Playing {
            return;
        }

        self.dir = self.next_dir;
        let head = self.snake[0];
        let new_head = (head.0 + self.dir.0, head.1 + self.dir.1);

        // Wall collision
        if new_head.0 < 0 || new_head.0 >= GRID_WIDTH || new_head.1 < 0 || new_head.1 >= GRID_HEIGHT {
            self.state = GameState::GameOver;
            return;
        }

        // Self collision
        for segment in &self.snake {
            if *segment == new_head {
                self.state = GameState::GameOver;
                return;
            }
        }

        self.snake.insert(0, new_head);

        // Eat food
        if new_head == self.food {
            self.score += 10;
            if self.score > self.high_score {
                self.high_score = self.score;
            }
            // Move food
            let fx = ((self.food.0 * 7 + 3) % (GRID_WIDTH - 2)) + 1;
            let fy = ((self.food.1 * 11 + 5) % (GRID_HEIGHT - 2)) + 1;
            self.food = (fx, fy);
        } else {
            self.snake.pop();
        }
    }
}

impl Application for SnakeApp {
    fn title(&self) -> &str {
        "Snake Game"
    }

    fn render(
        &mut self,
        fb: &mut Framebuffer,
        client_x: isize,
        client_y: isize,
        client_w: usize,
        client_h: usize,
    ) {
        // Arena background
        fb.fill_rect(client_x, client_y, client_w, client_h, Color::from_rgb(15, 23, 42));

        let offset_x = client_x + 16;
        let offset_y = client_y + 36;
        let arena_w = (GRID_WIDTH as usize) * CELL_SIZE;
        let arena_h = (GRID_HEIGHT as usize) * CELL_SIZE;

        // Top HUD Header
        let score_str = format!("SCORE: {:04}  HIGH: {:04}", self.score, self.high_score);
        fb.draw_string(client_x + 16, client_y + 12, &score_str, Color::from_rgb(251, 191, 36));

        // Arena Border
        fb.fill_rect(offset_x - 2, offset_y - 2, arena_w + 4, arena_h + 4, Color::from_rgb(51, 65, 85));
        fb.fill_rect(offset_x, offset_y, arena_w, arena_h, Color::from_rgb(11, 15, 25));

        // Draw Food (Apple)
        let food_x = offset_x + (self.food.0 as isize * CELL_SIZE as isize);
        let food_y = offset_y + (self.food.1 as isize * CELL_SIZE as isize);
        fb.fill_rect(food_x + 2, food_y + 2, CELL_SIZE - 4, CELL_SIZE - 4, Color::from_rgb(239, 68, 68));

        // Draw Snake
        for (i, segment) in self.snake.iter().enumerate() {
            let seg_x = offset_x + (segment.0 as isize * CELL_SIZE as isize);
            let seg_y = offset_y + (segment.1 as isize * CELL_SIZE as isize);

            let seg_color = if i == 0 {
                Color::from_rgb(74, 222, 128) // Bright green head
            } else {
                Color::from_rgb(34, 197, 94) // Green body
            };

            fb.fill_rect(seg_x + 1, seg_y + 1, CELL_SIZE - 2, CELL_SIZE - 2, seg_color);
        }

        // Game Over Overlay
        if self.state == GameState::GameOver {
            let box_w = 200;
            let box_h = 60;
            let bx = offset_x + (arena_w as isize - box_w) / 2;
            let by = offset_y + (arena_h as isize - box_h) / 2;

            fb.fill_rect(bx, by, box_w as usize, box_h as usize, Color::from_argb(220, 15, 23, 42));
            fb.draw_rect(bx, by, box_w as usize, box_h as usize, Color::from_rgb(239, 68, 68));

            fb.draw_string(bx + 60, by + 16, "GAME OVER!", Color::from_rgb(239, 68, 68));
            fb.draw_string(bx + 20, by + 34, "Press Space to Restart", Color::from_rgb(241, 245, 249));
        }
    }

    fn on_key(&mut self, key: DecodedKey) {
        match key {
            DecodedKey::RawKey(code) => match code {
                KeyCode::ArrowUp => {
                    if self.dir.1 != 1 {
                        self.next_dir = (0, -1);
                    }
                }
                KeyCode::ArrowDown => {
                    if self.dir.1 != -1 {
                        self.next_dir = (0, 1);
                    }
                }
                KeyCode::ArrowLeft => {
                    if self.dir.0 != 1 {
                        self.next_dir = (-1, 0);
                    }
                }
                KeyCode::ArrowRight => {
                    if self.dir.0 != -1 {
                        self.next_dir = (1, 0);
                    }
                }
                _ => {}
            },
            DecodedKey::Unicode(c) => match c {
                'w' | 'W' => {
                    if self.dir.1 != 1 {
                        self.next_dir = (0, -1);
                    }
                }
                's' | 'S' => {
                    if self.dir.1 != -1 {
                        self.next_dir = (0, 1);
                    }
                }
                'a' | 'A' => {
                    if self.dir.0 != 1 {
                        self.next_dir = (-1, 0);
                    }
                }
                'd' | 'D' => {
                    if self.dir.0 != -1 {
                        self.next_dir = (1, 0);
                    }
                }
                ' ' | 'r' | 'R' => {
                    if self.state == GameState::GameOver {
                        self.reset();
                    }
                }
                _ => {}
            },
        }
    }

    fn on_mouse_click(&mut self, _local_x: isize, _local_y: isize, _left: bool) {
        if self.state == GameState::GameOver {
            self.reset();
        }
    }

    fn on_tick(&mut self) -> bool {
        if self.state != GameState::Playing {
            return false;
        }
        self.tick_timer += 1;
        // Game tick every 12 ticks at 100 Hz (~8 steps per second)
        if self.tick_timer >= 12 {
            self.tick_timer = 0;
            self.step();
            true
        } else {
            false
        }
    }
}
