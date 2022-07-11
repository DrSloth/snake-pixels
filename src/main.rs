use std::{
    ops::{Add, AddAssign},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use image::{ImageBuffer, Rgba};
use imageproc::{drawing, rect::Rect};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub type Frame<'a> = ImageBuffer<Rgba<u8>, &'a mut [u8]>;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 800;
const FIELD_SIZE: u32 = 20;
const SNAKE_SIZE: u32 = WIDTH / FIELD_SIZE;

const FPS: u32 = 10;

const BG_COLOR: Rgba<u8> = Rgba([0, 0, 0, 0xFF]);
const HEAD_COLOR: Rgba<u8> = Rgba([0, 0xFC, 0, 0xFF]);
const BODY_COLOR: Rgba<u8> = Rgba([0, 0xFF, 0, 0xFF]);
const FRUIT_COLOR: Rgba<u8> = Rgba([0xFF, 0, 0, 0xFF]);

fn main() {
    run().unwrap();
}

fn run() -> Result<(), pixels::Error> {
    let event_loop = EventLoop::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Snake")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture)?
    };

    let rng = Rng::new_seeded();
    let mut interval = Interval::new(FPS);
    let mut world = World::new(rng);

    event_loop.run(move |event, _, control| {
        // Draw current frame
        if let Event::RedrawRequested(_) = event {
            world.draw(pixels.get_frame());
            pixels.render().unwrap();
        }

        // handle inputs
        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    *control = ControlFlow::Exit;
                    return;
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode: Some(virtual_keycode),
                            ..
                        },
                    ..
                } => {
                    world.input(virtual_keycode);
                }
                WindowEvent::Resized(size) => pixels.resize_surface(size.width, size.height),
                _ => (),
            },
            _ => (),
        }

        if interval.elapsed(control) {
            if world.update(control) {
                window.request_redraw();
            }
        }
    });
}

pub struct World {
    snake_head: Vector2d,
    snake_body: Vec<Vector2d>,
    fruit: Vector2d,
    dir: Vector2d,
    rng: Rng,
}

impl World {
    pub fn new(rng: Rng) -> Self {
        let mut me = Self {
            snake_head: Vector2d::new(FIELD_SIZE as i32 / 2, FIELD_SIZE as i32 / 2),
            snake_body: Vec::with_capacity(20),
            fruit: Vector2d::default(),
            dir: Vector2d::new(0, 0),
            rng,
        };

        me.create_fruit();
        me
    }

    pub fn input(&mut self, key: VirtualKeyCode) {
        self.dir = match key {
            VirtualKeyCode::Up | VirtualKeyCode::W => Vector2d::new(0, -1),
            VirtualKeyCode::Left | VirtualKeyCode::A => Vector2d::new(-1, 0),
            VirtualKeyCode::Down | VirtualKeyCode::S => Vector2d::new(0, 1),
            VirtualKeyCode::Right | VirtualKeyCode::D => Vector2d::new(1, 0),
            _ => self.dir,
        }
    }

    pub fn update(&mut self, flow: &mut ControlFlow) -> bool {
        if self.dir == Vector2d::new(0, 0) {
            return false;
        }

        if !self.snake_body.is_empty() {
            self.snake_body.rotate_right(1);
            self.snake_body[0] = self.snake_head;
        }
        self.snake_head += self.dir;

        if self.snake_head == self.fruit {
            let new_body = self.snake_body.last().copied().unwrap_or(self.snake_head);
            self.snake_body
                .push(new_body + Vector2d::new(-self.dir.x, -self.dir.y));
            self.create_fruit();
        }

        if !(0..FIELD_SIZE as i32).contains(&self.snake_head.x)
            || !(0..FIELD_SIZE as i32).contains(&self.snake_head.y)
            || self.snake_body.contains(&self.snake_head)
        {
            *flow = ControlFlow::Exit;
        }

        true
    }

    pub fn draw(&mut self, frame: &mut [u8]) {
        let mut frame = Frame::from_raw(WIDTH, HEIGHT, frame).unwrap();
        // clear background
        for pixel in frame.pixels_mut() {
            *pixel = BG_COLOR;
        }

        // draw border
        let border_rect = Rect::at(0, 0).of_size(WIDTH - 1, HEIGHT - 1);
        drawing::draw_hollow_rect_mut(&mut frame, border_rect, Rgba([0xFF, 0, 0, 0xFF]));

        // draw player
        let head = self.snake_head;
        drawing::draw_filled_rect_mut(&mut frame, snake_rect(head.x, head.y), HEAD_COLOR);
        for body in &self.snake_body {
            drawing::draw_filled_rect_mut(&mut frame, snake_rect(body.x, body.y), BODY_COLOR)
        }

        // draw fruit
        let fruit = self.fruit;
        drawing::draw_filled_rect_mut(&mut frame, snake_rect(fruit.x, fruit.y), FRUIT_COLOR);
    }

    fn create_fruit(&mut self) {
        self.fruit = Vector2d::new(self.random_pos(), self.random_pos());
        while self.fruit == self.snake_head || self.snake_body.contains(&self.fruit) {
            self.fruit = Vector2d::new(self.random_pos(), self.random_pos());
        }
    }

    fn random_pos(&mut self) -> i32 {
        (self.rng.gen() % FIELD_SIZE) as i32
    }
}

fn snake_rect(x: i32, y: i32) -> Rect {
    Rect::at(x * SNAKE_SIZE as i32, y * SNAKE_SIZE as i32).of_size(SNAKE_SIZE, SNAKE_SIZE)
}

/// A 2d point or direction
#[derive(Clone, Copy, Debug, Hash, Default, PartialEq)]
pub struct Vector2d {
    pub x: i32,
    pub y: i32,
}

impl Vector2d {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

impl Add for Vector2d {
    type Output = Self;
    fn add(self, other: Self) -> Self::Output {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl AddAssign for Vector2d {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    last: Instant,
    frame_duration: Duration,
}

impl Interval {
    pub fn new(fps: u32) -> Self {
        Self {
            last: Instant::now(),
            frame_duration: Duration::from_secs_f64(1f64 / fps as f64),
        }
    }

    // Check if the given frame is already over
    pub fn elapsed(&mut self, flow: &mut ControlFlow) -> bool {
        let now = Instant::now();
        let since_last = now.duration_since(self.last);
        let el = since_last > self.frame_duration;
        if el {
            self.last = Instant::now();
        } else {
            *flow = ControlFlow::WaitUntil(now + (self.frame_duration - since_last));
        }

        el
    }
}

#[derive(Debug, Clone)]
pub struct Rng {
    last: u32,
}

impl Rng {
    const MODULE: u32 = 1 << 31;

    pub fn new(seed: u32) -> Self {
        Self { last: seed }
    }

    pub fn new_seeded() -> Self {
        Self {
            last: (SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs()
                .wrapping_add(SNAKE_SIZE as u64 * WIDTH as u64)
                % Self::MODULE as u64) as u32,
        }
    }

    pub fn gen(&mut self) -> u32 {
        const MULTIPLIER: u32 = 1103515245;
        const INCREMENT: u32 = 12345;

        self.last = MULTIPLIER.wrapping_mul(self.last).wrapping_add(INCREMENT) % Self::MODULE;
        self.last
    }
}

impl Default for Rng {
    fn default() -> Self {
        Self {
            last: 98734677 + SNAKE_SIZE * WIDTH,
        }
    }
}
