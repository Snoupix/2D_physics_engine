use std::f64::consts::PI;
use std::thread::sleep;
use std::time::{Duration, Instant};

use eframe::egui::{self, layers::ShapeIdx};
use eframe::epaint::{CircleShape, Color32, Pos2, Rect, RectShape, Rounding, Shape, Stroke, Vec2};
use eframe::NativeOptions;
use rand::rngs::ThreadRng;
use rand::Rng;

// const DEFAULT_SCREEN_WIDTH: f32 = 1920.;
// const DEFAULT_SCREEN_HEIGHT: f32 = 1080.;
const WINDOW_SIZE: Vec2 = Vec2 { x: 1280., y: 720. };
const WINDOW_POS: Pos2 = Pos2 { x: 0., y: 0. };
const MAP_SIZE: Vec2 = Vec2 { x: 500., y: 500. };
const RECT_CANVAS_START: Pos2 = Pos2 { x: 50., y: 50. };
const RECT_CANVAS_END: Pos2 = Pos2 { x: 1050., y: 550. };
const CIRCLE_STARTING_POS: Pos2 = Pos2 {
    x: RECT_CANVAS_START.x * 2.,
    y: RECT_CANVAS_START.y * 2.,
};

const CIRCLES_NUMBER: u32 = 750;
const CIRCLES_MIN_RADIUS: f32 = 5.;
const CIRCLES_MAX_RADIUS: f32 = 15.;
const GRAVITY: Vec2 = Vec2 { x: 0., y: 0.1 };
const SLEEPING_FRAME_MS: u64 = 1;
const MAX_FPS: i32 = 144;
const SUB_STEPS: i32 = 10;

#[derive(Clone, Copy)]
struct Entity {
    id: u64,
    shape_id: ShapeIdx,
    position: Pos2,
    old_position: Pos2,
    acceleration: Vec2,
    color: Color32,
    radius: f32,
}

impl PartialEq for Entity {
    fn eq(&self, other: &Self) -> bool {
        self.shape_id == other.shape_id
    }
}

impl Eq for Entity {}

impl Entity {
    fn update(&mut self) {
        let velocity = self.position - self.old_position;
        self.old_position = self.position;
        self.apply_gravity();
        self.position = self.position + velocity + self.acceleration;
        self.acceleration = Vec2::new(0., 0.);
    }

    fn apply_gravity(&mut self) {
        self.accelerate(GRAVITY);
    }

    fn accelerate(&mut self, acc: Vec2) {
        self.acceleration += acc;
    }

    fn solve_collision(&mut self, other: &mut Self) {
        let response_coef: f32 = 0.75;
        let dist_pos = self.position - other.position;
        let dist2 = dist_pos.x.powi(2) + dist_pos.y.powi(2);
        let min_dist = self.radius + other.radius;

        if dist2 < min_dist.powi(2) {
            let dist = f32::sqrt(dist2);
            let n = dist_pos / dist;
            let mass_ratio_1 = self.radius / (self.radius + other.radius);
            let mass_ratio_2 = other.radius / (self.radius + other.radius);
            let delta = 0.5 * response_coef * (dist - min_dist);

            // self.old_position = self.position;
            // other.old_position = other.position;

            self.position -= n * (mass_ratio_2 * delta);
            other.position += n * (mass_ratio_1 * delta);
        }
    }

    fn apply_circle_contraint(&mut self) {
        let constraint_center = Pos2 {
            x: (RECT_CANVAS_START.x + RECT_CANVAS_END.x) / 2.,
            y: (RECT_CANVAS_START.y + RECT_CANVAS_END.y) / 2.,
        };
        let v = constraint_center - self.position;
        let dist = f32::sqrt(v.x * v.x + v.y * v.y);
        let canvas_radius = 300.;
        if dist > (canvas_radius - self.radius) {
            let n = v / dist;
            self.position = constraint_center - n * (canvas_radius - self.radius);
        }
    }

    fn apply_contraint(&mut self) {
        // down
        if self.position.y + self.radius > RECT_CANVAS_END.y {
            self.old_position = self.position;
            self.position = Pos2 {
                x: self.position.x,
                y: (self.position.y - self.radius) - (self.position.y - RECT_CANVAS_END.y),
            };
        }

        // up
        if self.position.y - self.radius < RECT_CANVAS_START.y {
            self.old_position = self.position;
            self.position = Pos2 {
                x: self.position.x,
                y: (self.position.y + self.radius) + (RECT_CANVAS_START.y - self.position.y),
            };
        }

        // right
        if self.position.x + self.radius > RECT_CANVAS_END.x {
            self.old_position = self.position;
            self.position = Pos2 {
                x: (self.position.x - self.radius) - (self.position.x - RECT_CANVAS_END.x),
                y: self.position.y,
            };
        }

        // left
        if self.position.x - self.radius < RECT_CANVAS_START.x {
            self.old_position = self.position;
            self.position = Pos2 {
                x: (self.position.x + self.radius) + (RECT_CANVAS_START.x - self.position.x),
                y: self.position.y,
            };
        }
    }
}

struct App {
    thread_rng: ThreadRng,
    next_entity_id: u64,
    entities: Vec<Entity>,
    pub map: Vec<Vec<u32>>,
    pub map_size: Vec2,
}

impl App {
    fn new() -> Self {
        Self {
            thread_rng: rand::thread_rng(),
            next_entity_id: 1,
            entities: Vec::new(),
            map_size: MAP_SIZE,
            map: (0..MAP_SIZE.x as _)
                .map(|_| (0..MAP_SIZE.y as _).collect())
                .collect(),
        }
    }

    fn create_circles(&mut self, ui: &mut egui::Ui) {
        if self.entities.len() == CIRCLES_NUMBER as usize {
            return;
        }

        let position = Pos2 {
            x: CIRCLE_STARTING_POS.x + self.next_entity_id as f32 * 5. % 500.,
            ..CIRCLE_STARTING_POS
        };
        // let radius = CIRCLES_MAX_RADIUS;
        let radius = self
            .thread_rng
            .gen_range(CIRCLES_MIN_RADIUS..=CIRCLES_MAX_RADIUS);
        let color = self.get_rainbow(self.next_entity_id as f32);

        self.entities.push(Entity {
            id: self.next_entity_id,
            shape_id: ui.painter().add(Shape::Circle(CircleShape {
                center: position,
                radius,
                fill: color,
                stroke: Stroke {
                    width: 0.,
                    color: Color32::WHITE,
                },
            })),
            position,
            old_position: position,
            acceleration: Vec2::default(),
            radius,
            color,
        });

        self.next_entity_id += 1;
    }

    fn draw_cricles(&self, ui: &mut egui::Ui) {
        for e in self.entities.iter() {
            ui.painter().add(Shape::Circle(CircleShape {
                center: Pos2 {
                    x: e.position.x,
                    y: e.position.y,
                },
                radius: e.radius,
                fill: e.color,
                stroke: Stroke {
                    width: 0.,
                    color: e.color,
                },
            }));
        }
    }

    fn draw_rect_canvas(&self, ui: &mut egui::Ui) {
        ui.painter().add(Shape::Rect(RectShape {
            rect: Rect {
                min: RECT_CANVAS_START,
                max: RECT_CANVAS_END,
            },
            rounding: Rounding::none().at_least(5.),
            fill: Color32::TRANSPARENT,
            stroke: Stroke {
                width: 5.,
                color: Color32::TRANSPARENT,
            },
        }));
    }

    fn update_entities(&mut self) {
        for i in 0..self.entities.len() {
            let (entity, entities) = self.entities[i..].split_first_mut().unwrap();
            for entity2 in entities {
                entity.solve_collision(entity2);
            }

            let entity = self.entities.get_mut(i).unwrap();

            entity.apply_circle_contraint();
            // entity.apply_contraint();
            entity.update();
        }
    }

    fn get_random_rgb(&self) -> Color32 {
        let (r, g, b) = rand::random::<(u8, u8, u8)>();
        Color32::from_rgb(r, g, b)
    }

    fn get_rainbow(&self, i: f32) -> Color32 {
        Color32::from_rgb(
            (255. * (f32::sin(i)).powi(2)) as u8,
            (255. * (f32::sin(i + 0.33 * 2.0 * PI as f32)).powi(2)) as u8,
            (255. * (f32::sin(i + 0.66 * 2.0 * PI as f32)).powi(2)) as u8,
        )
    }
}

struct Window {
    app: App,
    frame_time: Instant,
    frames: u64,
}

impl Window {
    fn new(_cc: &eframe::CreationContext<'_>, app: App) -> Self {
        Self {
            app,
            frames: 0,
            frame_time: Instant::now(),
        }
    }
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let elapsed = self.frame_time.elapsed().as_secs();
        let fps = self.frames / if elapsed == 0 { 1 } else { elapsed };
        self.frames += 1;

        egui::TopBottomPanel::top("app state").show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("{} entities", self.app.entities.len()))
                    .color(Color32::WHITE)
                    .size(12.)
                    .strong(),
            );
            ui.label(
                egui::RichText::new(format!("{fps} FPS"))
                    .color(Color32::WHITE)
                    .size(12.)
                    .strong(),
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                self.app.draw_rect_canvas(ui);

                if self.frames % 4 == 0 {
                    self.app.create_circles(ui);
                }

                self.app.draw_cricles(ui);

                for _ in 0..2 {
                    self.app.update_entities();
                }
            });
        });

        if ctx.input(|i| i.keys_down.get(&egui::Key::Escape).is_some()) {
            frame.close();
            std::process::exit(0);
        }

        sleep(Duration::from_millis(SLEEPING_FRAME_MS));
        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let app = App::new();

    let native_options = NativeOptions {
        always_on_top: false,
        maximized: false,
        decorated: true,
        drag_and_drop_support: false,
        icon_data: None,
        initial_window_pos: Some(WINDOW_POS),
        initial_window_size: Some(WINDOW_SIZE),
        min_window_size: None,
        max_window_size: None,
        resizable: true,
        transparent: false,
        mouse_passthrough: false,
        vsync: true,
        ..NativeOptions::default()
    };

    eframe::run_native(
        "Physics engine",
        native_options,
        Box::new(|cc| Box::new(Window::new(cc, app))),
    )
}
