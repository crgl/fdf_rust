use quicksilver::{
    Result,
    combinators::*,
    geom::{Shape, Vector},
    graphics::{Background::Img, Image, Color, PixelFormat},
    lifecycle::{Asset, Settings, State, Window, Event, run},
    input::{Key, MouseButton, ButtonState},
    load_file,
};

const WIDTH : f32  = 800f32;
const HEIGHT : f32 = 600f32;
const HRSPEED : f32 = 0.05f32;
const VRSPEED : f32 = 0.02f32;

struct Base {
    origin: Vector,
    center: Vector,
    dist: f32,
    stretch: f32,
}

impl Base {
    fn new() -> Self {
        Base {
            origin: Vector::new(0.5f32, 0.5f32),
            center: Vector::new(0.5f32, 0.5f32),
            dist: 2f32,
            stretch: 0.25f32,
        }
    }
}

struct Perspective {
    clicked: bool,
    base: Base,
    center: Vector,
    angles: (f32, f32), // (theta, phi)
    dist: f32,
    stretch: f32,
}

impl Perspective {
    fn new() -> Self {
        Perspective {
            clicked: false,
            base: Base::new(),
            center: Vector::new(0.5f32, 0.5f32),
            angles: (std::f32::consts::FRAC_PI_3, std::f32::consts::FRAC_PI_6),
            dist: 2f32,
            stretch: 0.25f32,
        }
    }
}

struct FdfMap {
    points: Vec<f32>,
    width: usize,
    z_range: f32,
    settings: Perspective,
    img_buffer: Vec<u8>,
}

impl FdfMap {
    fn update(&mut self) {}
}

struct FDF {
    fdf_map: Asset<FdfMap>,
}

fn fdf_parse(rect: String) -> FdfMap {
    let img_buffer: Vec<u8> = std::iter::repeat(0).take((WIDTH * HEIGHT) as usize * 4).collect();
    let mut maximum = std::f32::NEG_INFINITY;
    let mut minimum = std::f32::INFINITY;
    let mut width = None;
    let mut points = Vec::new();
    for (i, line) in rect.lines().enumerate() {
        let row: Vec<f32> = line.split_whitespace().map(|x| x.parse::<f32>().expect("Not a valid float")).collect();
        if let Some(w) = width {
            if w != row.len() {
                panic!("Invalid rectangle: row {} has length {} instead of {}", i, row.len(), w);
            }
        } else {
            width = Some(row.len());
        }
        for &num in row.iter() {
            maximum = f32::max(maximum, num);
            minimum = f32::min(minimum, num);
        }
        points.extend_from_slice(&row);
    }
    for point in points.iter_mut() {
        *point += minimum;
    }
    let width = width.expect("Please input at least one number");
    let z_range = maximum - minimum;
    FdfMap {
        points,
        width,
        z_range,
        settings: Perspective::new(),
        img_buffer,
    }
}

impl State for FDF {
    fn new() -> Result<FDF> {
        let fdf_map = Asset::new(load_file("42.fdf")
            .and_then(|contents| ok(String::from_utf8(contents).expect("The file must be UTF-8")))
            .and_then(|rect| ok(fdf_parse(rect))));
        Ok(FDF { fdf_map })
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        self.fdf_map.execute(|fdf_map| {
            let keyboard = window.keyboard();
            let mouse = window.mouse();
            let bottom_left = Vector::new(0f32, 0f32);
            let top_right = Vector::new(1f32, 1f32);
            let settings = &mut fdf_map.settings;
            if settings.clicked {
                if keyboard[Key::LShift].is_down() || keyboard[Key::RShift].is_down() {
                    settings.dist = f32::max(settings.base.dist + (mouse.pos().y - settings.base.origin.y) / HEIGHT, 1f32);
                } else if keyboard[Key::LControl].is_down() || keyboard[Key::RControl].is_down() {
                    settings.stretch = settings.base.stretch + (mouse.pos().y - settings.base.origin.y) / HEIGHT;
                } else {
                    settings.center = (settings.base.center + Vector::new((mouse.pos().x - settings.base.origin.x) / WIDTH, (mouse.pos().y - settings.base.origin.y) / HEIGHT)).clamp(bottom_left, top_right);
                }
            }
            let two_pi = 2f32 * std::f32::consts::PI;
            if keyboard[Key::Left].is_down() {
                settings.angles.0 -= HRSPEED;
                if settings.angles.0.is_sign_negative() {
                    settings.angles.0 += two_pi;
                }
            }
            if keyboard[Key::Right].is_down() {
                settings.angles.0 += HRSPEED;
                if (settings.angles.0 - two_pi).is_sign_positive() {
                    settings.angles.0 -= two_pi;
                }
            }
            if keyboard[Key::Up].is_down() {
                settings.angles.1 += VRSPEED;
                if (settings.angles.1 - two_pi).is_sign_positive() {
                    settings.angles.1 -= two_pi
                }
            }
            if keyboard[Key::Down].is_down() {
                settings.angles.1 -= VRSPEED;
                if settings.angles.0.is_sign_negative() {
                    settings.angles.0 += two_pi;
                }
            }
            Ok(())
        })
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        use MouseButton::*;
        use ButtonState::*;

        self.fdf_map.execute(|fdf_map| {
            if let Event::MouseButton(Left, Pressed) = event {
                let settings = &mut fdf_map.settings;
                settings.clicked = !settings.clicked;
                settings.base.origin = window.mouse().pos();
                settings.base.center = settings.center;
                settings.base.dist = settings.dist;
                settings.base.stretch = settings.stretch;
            }
            Ok(())
        })
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        self.fdf_map.execute(|fdf_map| {
            window.clear(Color::BLACK)?;
            fdf_map.update();
            let image = Image::from_raw(&fdf_map.img_buffer, WIDTH as u32, HEIGHT as u32, PixelFormat::RGBA)?;
            window.draw(&image.area().with_center((WIDTH / 2., HEIGHT / 2.)), Img(&image));
            Ok(())
        })
    }
}

fn main() {
    run::<FDF>("Draw Geometry", Vector::new(WIDTH, HEIGHT), Settings::default());
}