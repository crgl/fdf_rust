use quicksilver::{
    combinators::*,
    geom::{Shape, Vector},
    graphics::{Background::Img, Color, Image, PixelFormat},
    input::{ButtonState, Key, MouseButton},
    lifecycle::{run, Asset, Event, Settings, State, Window},
    load_file, Result,
};

const WIDTH: f32 = 1024f32;
const UWIDTH: usize = 1024;
const ROWLEN: usize = 4096;
const HEIGHT: f32 = 1024f32;
const UHEIGHT: usize = 1024;

const HRSPEED: f32 = 0.05f32;
const VRSPEED: f32 = 0.02f32;
const CAM2SCREEN: f32 = 2f32;

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
    base: Base,
    center: Vector,
    angles: (f32, f32), // (theta, phi)
    dist: f32,
    stretch: f32,
}

impl Perspective {
    fn new() -> Self {
        Perspective {
            base: Base::new(),
            center: Vector::new(0.5f32, 0.5f32),
            angles: (
                15f32 * std::f32::consts::FRAC_PI_8,
                std::f32::consts::FRAC_PI_6,
            ),
            dist: 1f32,
            stretch: 0.25f32,
        }
    }
}

struct FdfMap {
    points: Vec<f32>,
    coords: Vec<(i32, i32)>,
    colors: Vec<[u8; 4]>,
    width: usize,
    settings: Perspective,
    img_buffer: Vec<u8>,
}

unsafe fn place_pixel(buf: &mut Vec<u8>, pos: usize, color: &[u8]) {
    buf.get_unchecked_mut(pos..pos + 4)
        .clone_from_slice(&color[..4]);
}

fn clear_buf(buf: &mut Vec<u8>) {
    for byte in buf.iter_mut().skip(3).step_by(4) {
        *byte = 0;
    }
}

fn fix_point(p: &mut (i32, i32), m: f32, width: i32, height: i32) {
    let xshift = if p.0 < 1 {
        1 - p.0
    } else if p.0 > width - 2 {
        width - p.0 - 2
    } else {
        0
    };
    p.0 += xshift;
    p.1 += f32::round(m * (xshift as f32)) as i32;
    let yshift = if p.1 < 1 {
        1 - p.1
    } else if p.1 > height - 2 {
        height - p.1 - 2
    } else {
        0
    };
    p.1 += yshift;
    p.0 += f32::round((yshift as f32) / m) as i32;
}

fn place_line(
    mut p1: (i32, i32),
    mut p2: (i32, i32),
    c1: &[u8; 4],
    c2: &[u8; 4],
    buf: &mut Vec<u8>,
) {
    if (p1.0 < 0 || p1.0 >= UWIDTH as i32 || p1.1 < 0 || p1.1 >= UHEIGHT as i32)
        && (p2.0 < 0 || p2.0 >= UWIDTH as i32 || p2.1 < 0 || p2.1 >= UHEIGHT as i32)
    {
        return; // Both points are offscreen, just ignore them
    }
    match (p1.0 == p2.0, p1.1 == p2.1) {
        (true, _) => {
            if p1.0 < 0 || p1.0 >= UWIDTH as i32 {
                return;
            }
            let y1 = 0.max(((UHEIGHT - 1) as i32).min(p1.1)) as usize;
            let y2 = 0.max(((UHEIGHT - 1) as i32).min(p2.1)) as usize;
            if y1 < y2 {
                let mut pos = 4 * (y1 * UWIDTH + (p1.0 as usize));
                unsafe {
                    for y in y1..y2 {
                        let color: Vec<u8> = c1
                            .iter()
                            .zip(c2.iter())
                            .map(|(&v1, &v2)| {
                                ((usize::from(v1) * (y - y1) + usize::from(v2) * (y2 - y))
                                    / (y2 - y1)) as u8
                            })
                            .collect();
                        place_pixel(buf, pos, &color);
                        pos += ROWLEN;
                    }
                }
            } else {
                let mut pos = 4 * (y2 * UWIDTH + (p1.0 as usize));
                unsafe {
                    for y in y2..y1 {
                        let color: Vec<u8> = c1
                            .iter()
                            .zip(c2.iter())
                            .map(|(&v1, &v2)| {
                                ((usize::from(v2) * (y - y2) + usize::from(v1) * (y1 - y))
                                    / (y1 - y2)) as u8
                            })
                            .collect();
                        place_pixel(buf, pos, &color);
                        pos += ROWLEN;
                    }
                }
            }
        }
        (_, true) => {
            if p1.1 < 0 || p1.1 >= UHEIGHT as i32 {
                return;
            }
            let x1 = 0.max(((UWIDTH - 1) as i32).min(p1.0)) as usize;
            let x2 = 0.max(((UWIDTH - 1) as i32).min(p2.0)) as usize;
            if x1 < x2 {
                let mut pos = 4 * ((p1.1 as usize) * UWIDTH + x1);
                unsafe {
                    for x in x1..x2 {
                        let color: Vec<u8> = c1
                            .iter()
                            .zip(c2.iter())
                            .map(|(&v1, &v2)| {
                                ((usize::from(v1) * (x - x1) + usize::from(v2) * (x2 - x))
                                    / (x2 - x1)) as u8
                            })
                            .collect();
                        place_pixel(buf, pos, &color);
                        pos += 4;
                    }
                }
            } else {
                let mut pos = 4 * ((p1.1 as usize) * UWIDTH + x2);
                unsafe {
                    for x in x2..x1 {
                        let color: Vec<u8> = c1
                            .iter()
                            .zip(c2.iter())
                            .map(|(&v1, &v2)| {
                                ((usize::from(v2) * (x - x2) + usize::from(v1) * (x1 - x))
                                    / (x1 - x2)) as u8
                            })
                            .collect();
                        place_pixel(buf, pos, &color);
                        pos += 4;
                    }
                }
            }
        }
        _ => {
            let m = ((p2.1 - p1.1) as f32) / ((p2.0 - p1.0) as f32);
            fix_point(&mut p1, m, UWIDTH as i32, UHEIGHT as i32);
            fix_point(&mut p2, m, UWIDTH as i32, UHEIGHT as i32);
            let count = std::cmp::max((p2.0 - p1.0).abs(), (p2.1 - p1.1).abs());
            let urange: f32 = (p2.0 - p1.0) as f32;
            let vrange: f32 = (p2.1 - p1.1) as f32;
            let u0: f32 = p1.0 as f32;
            let v0: f32 = p1.1 as f32;
            unsafe {
                for inc in 0..=count {
                    let fraction = (inc as f32) / (count as f32);
                    let color: Vec<u8> = c1
                        .iter()
                        .zip(c2.iter())
                        .map(|(&v1, &v2)| {
                            ((v1 as f32) * fraction + (v2 as f32) * (1f32 - fraction)) as u8
                        })
                        .collect();
                    let pos = (f32::round(v0 + fraction * vrange) * WIDTH
                        + f32::round(u0 + fraction * urange))
                        as usize
                        * 4;
                    place_pixel(buf, pos, &color);
                }
            }
        }
    }
}

impl FdfMap {
    fn project(&mut self) {
        let width = self.width as f32;
        let height = (self.points.len() / self.width) as f32;
        let mut x = 0;
        let mut y = 0;
        for (i, &z) in self.points.iter().enumerate() {
            let fx = (x as f32) / width - self.settings.center.x;
            let fy = self.settings.center.y - (y as f32) / height;
            let theta = self.settings.angles.0;
            let phi = self.settings.angles.1;
            let k = 1f32
                + ((fx * theta.sin() + fy * theta.cos()) * phi.cos() + self.settings.dist)
                    / CAM2SCREEN;
            let w_shift = (fx * theta.cos() - fy * theta.sin()) / k;
            let h_shift = ((fx * theta.sin() + fy * theta.cos()) * phi.sin()
                + z * self.settings.stretch * phi.cos())
                / k;
            let u = (WIDTH * (0.5f32 + 0.8f32 * w_shift)) as i32;
            let v = (HEIGHT * (0.5f32 - 0.8f32 * h_shift)) as i32;
            let red = (80f32 * (1.2f32 - z)) as u8;
            let green = (80f32 * (2f32 * z + 1f32)) as u8;
            let blue = (80f32 * (z + 2f32)) as u8;
            let alpha = (180f32 / k) as u8;
            self.colors[i].clone_from_slice(&[red, green, blue, alpha]);
            self.coords[i] = (u, v);
            x += 1;
            if x == self.width {
                y += 1;
                x = 0;
            }
        }
    }

    fn update(&mut self) {
        self.project();
        let points = &self.coords;
        clear_buf(&mut self.img_buffer);
        for (i, (&p1, &p2)) in points.iter().zip(points.iter().skip(1)).enumerate() {
            if (i + 1) % self.width != 0 {
                place_line(
                    p1,
                    p2,
                    &self.colors[i + 1],
                    &self.colors[i],
                    &mut self.img_buffer,
                );
            }
        }
        for (i, (&p1, &p2)) in points
            .iter()
            .zip(points.iter().skip(self.width))
            .enumerate()
        {
            place_line(
                p1,
                p2,
                &self.colors[i + self.width],
                &self.colors[i],
                &mut self.img_buffer,
            );
        }
    }
}

struct FDF {
    fdf_map: Asset<FdfMap>,
}

fn fdf_parse(rect: String) -> FdfMap {
    let mut img_buffer: Vec<u8> = Vec::with_capacity(UWIDTH * UHEIGHT * 4);
    for i in 0..UHEIGHT {
        for j in 0..UWIDTH {
            let w = (j * 255 / UWIDTH) as u8;
            let h = (i * 255 / UHEIGHT) as u8;
            img_buffer.push(0);
            img_buffer.push(w);
            img_buffer.push(h);
            img_buffer.push(0);
        }
    }
    let mut maximum = std::f32::NEG_INFINITY;
    let mut minimum = std::f32::INFINITY;
    let mut width = None;
    let mut points = Vec::new();
    for (i, line) in rect.lines().enumerate() {
        let row: Vec<f32> = line
            .split_whitespace()
            .map(|x| x.parse::<f32>().expect("Not a valid float"))
            .collect();
        if let Some(w) = width {
            if w != row.len() {
                panic!(
                    "Invalid rectangle: row {} has length {} instead of {}",
                    i,
                    row.len(),
                    w
                );
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
    let coords: Vec<(i32, i32)> = std::iter::repeat((0, 0)).take(points.len()).collect();
    let colors: Vec<[u8; 4]> = coords.iter().map(|_| [0; 4]).collect();
    let width = width.expect("Please input at least one number");
    if width == 0 {
        panic!("Input must have at least one column")
    }
    let z_range = maximum - minimum;
    for point in points.iter_mut() {
        *point += minimum;
        *point /= z_range;
    }
    FdfMap {
        points,
        coords,
        colors,
        width,
        settings: Perspective::new(),
        img_buffer,
    }
}

impl State for FDF {
    fn new() -> Result<FDF> {
        let fdf_map = Asset::new(
            load_file("crgl.fdf")
                .and_then(|contents| {
                    ok(String::from_utf8(contents).expect("The file must be UTF-8"))
                })
                .and_then(|rect| ok(fdf_parse(rect))),
        );
        Ok(FDF { fdf_map })
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        self.fdf_map.execute(|fdf_map| {
            let keyboard = window.keyboard();
            let mouse = window.mouse();
            let bottom_left = Vector::new(0f32, 0f32);
            let top_right = Vector::new(1f32, 1f32);
            let settings = &mut fdf_map.settings;
            if mouse[MouseButton::Left].is_down() {
                if keyboard[Key::LShift].is_down() || keyboard[Key::RShift].is_down() {
                    settings.dist = f32::max(
                        settings.base.dist + (mouse.pos().y - settings.base.origin.y) / HEIGHT,
                        0.1f32,
                    );
                } else if keyboard[Key::LControl].is_down() || keyboard[Key::RControl].is_down() {
                    settings.stretch =
                        settings.base.stretch + (mouse.pos().y - settings.base.origin.y) / HEIGHT;
                } else {
                    settings.center = (settings.base.center
                        + Vector::new(
                            (mouse.pos().x - settings.base.origin.x) / WIDTH,
                            (mouse.pos().y - settings.base.origin.y) / HEIGHT,
                        ))
                    .clamp(bottom_left, top_right);
                }
            }
            let pi_two = std::f32::consts::FRAC_PI_2;
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
                if (settings.angles.1 - pi_two).is_sign_positive() {
                    settings.angles.1 = pi_two;
                }
            }
            if keyboard[Key::Down].is_down() {
                settings.angles.1 -= VRSPEED;
                if settings.angles.1.is_sign_negative() {
                    settings.angles.1 = 0f32;
                }
            }
            Ok(())
        })
    }

    fn event(&mut self, event: &Event, window: &mut Window) -> Result<()> {
        use ButtonState::*;
        use MouseButton::*;

        self.fdf_map.execute(|fdf_map| {
            if let Event::MouseButton(Left, Pressed) = event {
                let settings = &mut fdf_map.settings;
                settings.base.origin = window.mouse().pos();
                settings.base.center = settings.center;
                settings.base.dist = settings.dist;
                settings.base.stretch = settings.stretch;
            }
            if let Event::Key(Key::Space, Pressed) = event {
                fdf_map.settings = Perspective::new();
            }
            Ok(())
        })
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        self.fdf_map.execute(|fdf_map| {
            window.clear(Color::BLACK)?;
            fdf_map.update();
            let image = Image::from_raw(
                &fdf_map.img_buffer,
                UWIDTH as u32,
                UHEIGHT as u32,
                PixelFormat::RGBA,
            )?;
            window.draw(
                &image.area().with_center((WIDTH / 2., HEIGHT / 2.)),
                Img(&image),
            );
            Ok(())
        })
    }
}

fn main() {
    run::<FDF>("FDF", Vector::new(WIDTH, HEIGHT), Settings::default());
}
