use macroquad::prelude::*;

use std::{fs, str::FromStr};

const STARFIELD_FRAGMENT_SHADER: &str = include_str!("starfield-shader.glsl");
const STARFIELD_VERTEX_SHADER: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying float iTime;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec4 _Time;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    iTime = _Time.x;
}
";

const CRT_FRAGMENT_SHADER: &str = include_str!("crt-shader.glsl");
const CRT_VERTEX_SHADER:&str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

struct Shape {
    size: f32,
    speed: f32,
    x: f32,
    y: f32,
    collided: bool,
}

impl Shape {
    fn collides_with(&self, other: &Self) -> bool {
        self.rect().overlaps(&other.rect())
    }

    fn rect(&self) -> Rect {
        Rect {
            x: self.x - self.size / 2.0,
            y: self.y - self.size / 2.0,
            w: self.size,
            h: self.size,
        }
    }
}

enum GameState {
    MainMenu,
    Playing,
    Paused,
    GameOver,
}


fn window_conf() -> Conf {
    Conf {
        window_title: "Cathedral".to_string(),
        window_width: 800,
        window_height: 600,
        // high_dpi: todo!(),
        fullscreen: false,
        // sample_count: todo!(),
        window_resizable: true,
        // icon: todo!(),
        // platform: todo!(),
        ..Default::default()
    }
}

fn get_preferred_size(texel_size: u32) -> IVec2 {
    ivec2((screen_width() / texel_size as f32) as i32, (screen_height() / texel_size as f32) as i32)
}

#[macroquad::main(window_conf)]
async fn main() {
    let texel_size = 2;
    let mut pref_size: IVec2 = get_preferred_size(texel_size);


    let mut main_render_target = render_target(pref_size.x as u32, pref_size.y as u32);
    main_render_target.texture.set_filter(FilterMode::Nearest);

    const MOVEMENT_SPEED: f32 = 200.0;

    rand::srand(miniquad::date::now() as u64);
    let mut squares = vec![];
    let mut bullets: Vec<Shape> = vec![];
    let mut circle = Shape {
        size: 32.0,
        speed: MOVEMENT_SPEED,
        x: screen_width() / 2.0,
        y: screen_height() / 2.0,
        collided: false,
    };
    let mut score: u32 = 0;
    let mut high_score: u32 = fs::read_to_string("highscore.dat")
        .map_or(Ok(0), |i| i.parse::<u32>())
        .unwrap_or(0);
    let mut game_state = GameState::MainMenu;

    let mut direction_modifier: f32 = 0.0;

    let starfield_render_target = render_target(800, 600);
    starfield_render_target.texture.set_filter(FilterMode::Nearest);

    let starfield_material = load_material(
        ShaderSource::Glsl {
            vertex: STARFIELD_VERTEX_SHADER,
            fragment: STARFIELD_FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("iResolution", UniformType::Float2),
                UniformDesc::new("direction_modifier", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    let crt_material = load_material(
        ShaderSource::Glsl {
            vertex: CRT_VERTEX_SHADER,
            fragment: CRT_FRAGMENT_SHADER,
        },
        Default::default(),
    )
    .unwrap();

    loop {
        pref_size = get_preferred_size(texel_size);
        let pref_size_f32 = pref_size.as_vec2();

        let cur_target_size = main_render_target.texture.size().as_ivec2();

        if cur_target_size != pref_size {
            main_render_target = render_target(pref_size.x as u32, pref_size.y as u32);
        }

        set_camera(&Camera2D {
            zoom: vec2(1. / pref_size_f32.x * 2., 1. / pref_size_f32.y * 2.),
            target: vec2((pref_size_f32.x * 0.5f32).floor(), (pref_size_f32.y * 0.5f32).floor()),
            render_target: Some(main_render_target.clone()),
            ..Default::default()
        });
        clear_background(BLACK);

        starfield_material.set_uniform("iResolution", (pref_size_f32.x, pref_size_f32.y));
        starfield_material.set_uniform("direction_modifier", direction_modifier);
        gl_use_material(&starfield_material);
        draw_texture_ex(
            &starfield_render_target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                // dest_size: Some(vec2(screen_width(), screen_height())),
                dest_size: Some(vec2(pref_size_f32.x, pref_size_f32.y)),
                ..Default::default()
            },
        );
        gl_use_default_material();

        draw_line(0., 100., 100., 100., 2.0, BLUE);

        match game_state {
            GameState::MainMenu => {
                if is_key_pressed(KeyCode::Escape) {
                    std::process::exit(0);
                }
                if is_key_pressed(KeyCode::Space) {
                    squares.clear();
                    bullets.clear();
                    circle.x = pref_size_f32.x / 2.0;
                    circle.y = pref_size_f32.y / 2.0;
                    score = 0;
                    game_state = GameState::Playing;
                }
                let text = "Press space";
                let text_dimensions = measure_text(text, None, 50, 1.0);
                draw_text_ex(
                    text,
                    pref_size_f32.x / 2.0 - text_dimensions.width / 2.0,
                    pref_size_f32.y / 2.0,
                    TextParams {
                        font: None,
                        font_size: 32, font_scale: 1.0, font_scale_aspect: 1.0, rotation: 0., color: WHITE }
                );
            }
            GameState::Playing => {
                let delta_time = get_frame_time();
                if is_key_down(KeyCode::Right) {
                    circle.x += MOVEMENT_SPEED * delta_time;
                    direction_modifier += 0.05 * delta_time;
                }
                if is_key_down(KeyCode::Left) {
                    circle.x -= MOVEMENT_SPEED * delta_time;
                    direction_modifier -= 0.05 * delta_time;
                }
                if is_key_down(KeyCode::Down) {
                    circle.y += MOVEMENT_SPEED * delta_time;
                }
                if is_key_down(KeyCode::Up) {
                    circle.y -= MOVEMENT_SPEED * delta_time;
                }
                if is_key_pressed(KeyCode::Space) {
                    bullets.push(Shape {
                        x: circle.x,
                        y: circle.y,
                        speed: circle.speed * 2.0,
                        size: 5.0,
                        collided: false,
                    });
                }
                if is_key_pressed(KeyCode::Escape) {
                    game_state = GameState::Paused;
                }

                // Clamp X and Y to be within the screen
                circle.x = clamp(circle.x, 0.0, pref_size_f32.x);
                circle.y = clamp(circle.y, 0.0, pref_size_f32.x);

                // Generate a new square
                if rand::gen_range(0, 99) >= 95 {
                    let size = rand::gen_range(16.0, 64.0);
                    squares.push(Shape {
                        size,
                        speed: rand::gen_range(50.0, 150.0),
                        x: rand::gen_range(size / 2.0, pref_size_f32.x - size / 2.0),
                        y: -size,
                        collided: false,
                    });
                }

                // Movement
                for square in &mut squares {
                    square.y += square.speed * delta_time;
                }
                for bullet in &mut bullets {
                    bullet.y -= bullet.speed * delta_time;
                }

                // Remove shapes outside of screen
                squares.retain(|square| square.y < pref_size_f32.x + square.size);
                bullets.retain(|bullet| bullet.y > 0.0 - bullet.size / 2.0);

                // Remove collided shapes
                squares.retain(|square| !square.collided);
                bullets.retain(|bullet| !bullet.collided);

                // Check for collisions
                if squares.iter().any(|square| circle.collides_with(square)) {
                    if score == high_score {
                        fs::write("highscore.dat", high_score.to_string()).ok();
                    }
                    game_state = GameState::GameOver;
                }
                for square in squares.iter_mut() {
                    for bullet in bullets.iter_mut() {
                        if bullet.collides_with(square) {
                            bullet.collided = true;
                            square.collided = true;
                            score += square.size.round() as u32;
                            high_score = high_score.max(score);
                        }
                    }
                }

                // Draw everything
                for bullet in &bullets {
                    draw_circle(bullet.x, bullet.y, bullet.size / 2.0, RED);
                }
                draw_circle(circle.x, circle.y, circle.size / 2.0, YELLOW);
                for square in &squares {
                    draw_rectangle(
                        square.x - square.size / 2.0,
                        square.y - square.size / 2.0,
                        square.size,
                        square.size,
                        GREEN,
                    );
                }
                draw_text(
                    format!("Score: {}", score).as_str(),
                    16.0,
                    16.0,
                    16.0,
                    WHITE,
                );
                let highscore_text = format!("High score: {}", high_score);
                let text_dimensions = measure_text(highscore_text.as_str(), None, 16, 1.0);
                draw_text(
                    highscore_text.as_str(),
                    pref_size_f32.x - text_dimensions.width - 16.0,
                    16.0,
                    16.0,
                    WHITE,
                );
            }
            GameState::Paused => {
                if is_key_pressed(KeyCode::Space) {
                    game_state = GameState::Playing;
                }
                let text = "Paused";
                let text_dimensions = measure_text(text, None, 32, 1.0);
                draw_text(
                    text,
                    pref_size_f32.x / 2.0 - text_dimensions.width / 2.0,
                    pref_size_f32.y / 2.0,
                    32.0,
                    WHITE,
                );
            }
            GameState::GameOver => {
                if is_key_pressed(KeyCode::Space) {
                    game_state = GameState::MainMenu;
                }
                let text = "GAME OVER!";
                let text_dimensions = measure_text(text, None, 32, 1.0);
                draw_text(
                    text,
                    pref_size_f32.x / 2.0 - text_dimensions.width / 2.0,
                    pref_size_f32.y / 2.0,
                    32.0,
                    RED,
                );
            }
        }


        set_default_camera();
        clear_background(WHITE);
        gl_use_material(&crt_material);

        let screen_pad_x = (screen_width() - ((pref_size.x as f32) * (texel_size as f32))) * 0.5;
        let screen_pad_y = (screen_height() - ((pref_size.y as f32) * (texel_size as f32))) * 0.5;

        let dest_size = pref_size_f32 * vec2(texel_size as f32, texel_size as f32);

        draw_texture_ex(
            &main_render_target.texture,
            screen_pad_x,
            screen_pad_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(dest_size),
                ..Default::default()
            },
        );
        gl_use_default_material();

        draw_text(
            get_fps().to_string().as_str(),
            16.0,
            32.0,
            16.0,
            GOLD,
        );
        next_frame().await
    }
}