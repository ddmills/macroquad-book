use bevy_ecs::prelude::*;
use macroquad::{prelude::*, texture::RenderTarget};

use std::{collections::HashSet, fs};

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

const GLYPH_FRAGMENT_SHADER: &str = include_str!("glyph-shader.glsl");
const GLYPH_VERTEX_SHADER: &str = "#version 400
attribute vec3 position;
attribute vec2 texcoord;

varying lowp vec2 uv;
varying flat uint idx;

uniform mat4 Model;
uniform mat4 Projection;
uniform vec4 _idx;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    uv = texcoord;
    idx = uint(_idx.x);
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

#[derive(Resource, Default)]
struct GlyphMaterial {
    pub material: Option<Material>,
    pub texture: Option<Texture2D>
}

#[derive(Resource, Default)]
struct MainRenderTarget {
    pub target: Option<RenderTarget>,
}

#[derive(Resource, Default)]
struct CurrentState {
    pub previous: GameState,
    pub current: GameState,
    pub next: GameState,
}

#[derive(Resource, Default)]
struct Screen {
    pub width: usize,
    pub height: usize,
}

#[derive(Resource, Default)]
struct Time {
    pub dt: f32,
    pub fps: i32,
}

#[derive(Resource, Default)]
struct KeyInput {
    pub down: HashSet<KeyCode>,
    pub pressed: HashSet<KeyCode>,
}

impl KeyInput {
    pub fn is_down(&self, key:KeyCode) -> bool {
        self.down.contains(&key)
    }

    pub fn is_pressed(&self, key:KeyCode) -> bool {
        self.pressed.contains(&key)
    }
}

#[derive(Component)]
struct Player {
    pub speed: f32,
}

#[derive(Component)]
struct Faller {
    pub speed: f32,
}

#[derive(Component)]
struct Bullet {
    pub speed: f32,
}

#[derive(Component)]
struct Shape {
    size: f32,
    x: f32,
    y: f32,
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

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
    GameOver,
}

fn update_shapes(
    mut cmds: Commands,
    mut q_shapes: Query<(Entity, &Faller, &mut Shape)>,
    time: Res<Time>,
    screen: Res<Screen>,
) {
    for (entity, faller, mut shape) in q_shapes.iter_mut() {
        shape.y += faller.speed * time.dt;

        if shape.y > screen.height as f32 {
            cmds.entity(entity).despawn();
        }
    }
}

fn update_bullets(
    mut cmds: Commands,
    mut q_bullets: Query<(Entity, &Bullet, &mut Shape)>,
    time: Res<Time>,
) {
    for (entity, bullet, mut shape) in q_bullets.iter_mut() {
        shape.y -= bullet.speed * time.dt;

        if shape.y < 0. {
            cmds.entity(entity).despawn();
        }
    }
}

fn check_collisions(
    mut cmds: Commands,
    q_bullets: Query<(Entity, &Shape), With<Bullet>>,
    q_fallers: Query<(Entity, &Shape), With<Faller>>,
    q_player: Single<(Entity, &Shape), With<Player>>,
    mut state: ResMut<CurrentState>,
) {
    for (e_bullet, s_bullet) in q_bullets.iter() {
        for (e_faller, s_faller) in q_fallers.iter() {
            if s_bullet.collides_with(s_faller) {
                cmds.entity(e_bullet).despawn();
                cmds.entity(e_faller).despawn();
            }
        }
    }

    for (e_faller, s_faller) in q_fallers.iter() {
        if s_faller.collides_with(q_player.1) {
            cmds.entity(e_faller).despawn();
            state.next = GameState::GameOver;
        }
    }
}

fn spawn_shapes(mut cmds: Commands, screen: Res<Screen>) {
    if rand::gen_range(0, 99) >= 95 {
        let size = rand::gen_range(16.0, 64.0);

        let min_x = size / 2.;
        let max_x = screen.width as f32 - size / 2.;

        cmds.spawn((
            Shape {
                size,
                x: rand::gen_range(min_x, max_x),
                y: -size,
            },
            Faller {
                speed: rand::gen_range(50.0, 150.0),
            },
        ));
    }
}

fn update_time(mut time: ResMut<Time>) {
    time.dt = get_frame_time();
    time.fps = get_fps();
}

fn update_key_input(mut keys: ResMut<KeyInput>) {
    keys.down = get_keys_down();
    keys.pressed = get_keys_pressed();
}

fn update_screen(mut screen: ResMut<Screen>, mut main_render_target: ResMut<MainRenderTarget>) {
    let screen_size = get_preferred_size(2);
    screen.width = screen_size.x as usize;
    screen.height = screen_size.y as usize;

    let pref_size_f32 = screen_size.as_vec2();

    let Some(ref target) = main_render_target.target else {
        trace!("NO RENDER TARGET!");
        return;
    };

    // let cur_target_size = target.texture.size().as_ivec2();

    // let mut t = target.clone();

    // if cur_target_size != screen_size {
    //     let new_target = render_target(screen_size.x as u32, screen_size.y as u32);
    //     new_target.texture.set_filter(FilterMode::Nearest);
    //     t = new_target.clone();
    //     main_render_target.target = Some(new_target);
    // }

    // set_camera(&Camera2D {
    //     zoom: vec2(1. / pref_size_f32.x * 2., 1. / pref_size_f32.y * 2.),
    //     target: vec2((pref_size_f32.x * 0.5f32).floor(), (pref_size_f32.y * 0.5f32).floor()),
    //     render_target: Some(t.clone()),
    //     ..Default::default()
    // });
}

fn update_player(
    mut cmds: Commands,
    keys: Res<KeyInput>,
    q_player: Single<(&mut Shape, &Player)>,
    time: Res<Time>,
    screen: Res<Screen>,
) {
    let (mut shape, player) = q_player.into_inner();

    if keys.is_down(KeyCode::A) {
        shape.x -= player.speed * time.dt;
    }

    if keys.is_down(KeyCode::D) {
        shape.x += player.speed * time.dt;
    }

    if keys.is_down(KeyCode::W) {
        shape.y -= player.speed * time.dt;
    }

    if keys.is_down(KeyCode::S) {
        shape.y += player.speed * time.dt;
    }

    shape.x = clamp(shape.x, 0.0, screen.width as f32);
    shape.y = clamp(shape.y, 0.0, screen.height as f32);

    if keys.is_pressed(KeyCode::Space) {
        cmds.spawn((
            Bullet {
                speed: player.speed * 2.0,
            },
            Shape {
                x: shape.x,
                y: shape.y,
                size: 5.0,
            }
        ));
    }
}

fn update_main_menu(
    keys: Res<KeyInput>,
    mut state: ResMut<CurrentState>,
    screen: Res<Screen>,
) {
    if keys.is_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }

    if keys.is_pressed(KeyCode::Space) {
        // bullets.clear();
        // player.x = pref_size_f32.x / 2.0;
        // player.y = pref_size_f32.y / 2.0;
        // score = 0;
        state.next = GameState::Playing;
    }

    let text = "Press space";
    let text_dimensions = measure_text(text, None, 32, 1.0);

    draw_text_ex(
        text,
        screen.width as f32 / 2.0 - text_dimensions.width / 2.0,
        screen.height as f32 / 2.0,
        TextParams {
            font: None,
            font_size: 32,
            font_scale: 1.0,
            font_scale_aspect: 1.0,
            rotation: 0.,
            color: WHITE
        }
    );
}

fn update_paused(
    keys: Res<KeyInput>,
    mut state: ResMut<CurrentState>,
    screen: Res<Screen>,
) {
    if keys.is_pressed(KeyCode::Escape) {
        std::process::exit(0);
    }

    if keys.is_pressed(KeyCode::Space) {
        state.next = GameState::Playing;
    }

    let text = "Paused";
    let text_dimensions = measure_text(text, None, 32, 1.0);

    draw_text(
        text,
        screen.width as f32 / 2.0 - text_dimensions.width / 2.0,
        screen.height as f32 / 2.0,
        32.0,
        WHITE,
    );
}

fn update_game_over(
    keys: Res<KeyInput>,
    mut state: ResMut<CurrentState>,
    screen: Res<Screen>,
) {
    if keys.is_pressed(KeyCode::Space) {
        state.next = GameState::MainMenu;
    }

    let text = "GAME OVER!";
    let text_dimensions = measure_text(text, None, 16, 1.0);

    draw_text(
        text,
        screen.width as f32 / 2.0 - text_dimensions.width / 2.0,
        screen.height as f32 / 2.0,
        16.0,
        RED,
    );
}

fn update_playing(
    keys: Res<KeyInput>,
     mut state: ResMut<CurrentState>,
) {
    if keys.is_pressed(KeyCode::Escape) {
        state.next = GameState::Paused;
    }
}

fn in_state(state: GameState) -> impl Fn(Res<CurrentState>) -> bool {
    move |res| res.current == state && res.next == state && res.previous == state
}

fn enter_state(state: GameState) -> impl Fn(Res<CurrentState>) -> bool {
    move |res| res.current == state && res.previous != state
}

fn leave_state(state: GameState) -> impl Fn(Res<CurrentState>) -> bool {
    move |res| res.current == state && res.next != state
}

fn update_states(mut state: ResMut<CurrentState>) {
    state.previous = state.current;
    state.current = state.next;
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

fn render_fps(time: Res<Time>) {
    draw_text(
        time.fps.to_string().as_str(),
        16.0,
        32.0,
        16.0,
        GOLD,
    );
}

fn render_shapes(q_shapes: Query<&Shape>, mat: Res<GlyphMaterial>) {
    let material = mat.material.clone().unwrap();
    let texture = mat.texture.clone().unwrap();
    gl_use_material(&material);
    
    for shape in q_shapes.iter() {
        material.set_uniform("fg1", Color::from_rgba(10, 20, 255, 255));
        material.set_uniform("fg2", Color::from_rgba(10, 255, 30, 255));
        material.set_uniform("outline", Color::from_rgba(10, 255, 30, 255));
        material.set_uniform("bg", Color::from_rgba(10, 20, 30, 0));
        material.set_uniform("idx", 25u32);
        // draw_texture(&texture, shape.x, shape.y, WHITE);
        draw_texture_ex(&texture, shape.x, shape.y, WHITE, DrawTextureParams {
            dest_size: Some(vec2(shape.size, shape.size)),
            source: None,
            rotation: 0.,
            flip_x: false,
            flip_y: false,
            pivot: None,
        });
        // draw_rectangle(
        //     shape.x - shape.size / 2.0,
        //     shape.y - shape.size / 2.0,
        //     shape.size,
        //     shape.size,
        //     GREEN,
        // );
    }
    gl_use_default_material();
}

fn setup_player(mut cmds: Commands, screen: Res<Screen>) {
    trace!("Setup!");
    cmds.spawn((
        Player {
            speed: 200.,
        },
        Shape {
            size: 32.,
            x: screen.width as f32 / 2.0,
            y: screen.height as f32 / 2.0,
        },
    ));
}

fn teardown(
    mut cmds: Commands,
    q_shapes: Query<Entity, With<Shape>>,
) {
    trace!("Teardown!");
    for e in q_shapes.iter() {
        cmds.entity(e).despawn();
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();

    world.init_resource::<Time>();
    world.init_resource::<Screen>();
    world.init_resource::<KeyInput>();
    world.init_resource::<CurrentState>();
    world.init_resource::<GlyphMaterial>();
    world.init_resource::<MainRenderTarget>();

    let mut schedule = Schedule::default();
    let mut schedule_post_update = Schedule::default();

    schedule_post_update.add_systems(update_states);

    schedule.add_systems(
        (update_time, render_fps, update_key_input, update_screen).chain()
    );

    schedule.add_systems((
        update_main_menu.run_if(in_state(GameState::MainMenu)),
        update_paused.run_if(in_state(GameState::Paused)),
        update_game_over.run_if(in_state(GameState::GameOver)),
        setup_player.run_if(enter_state(GameState::Playing)),
        update_playing.run_if(in_state(GameState::Playing)),
        teardown.run_if(leave_state(GameState::MainMenu)),
    ).chain());

    schedule.add_systems(
        (
            check_collisions,
            spawn_shapes,
            update_player,
            update_shapes,
            update_bullets,
            render_shapes,
        ).run_if(in_state(GameState::Playing))
    );

    set_default_filter_mode(FilterMode::Nearest);
    let texel_size = 2;
    let mut pref_size: IVec2 = get_preferred_size(texel_size);

    let mut main_render_target = render_target(pref_size.x as u32, pref_size.y as u32);
    main_render_target.texture.set_filter(FilterMode::Nearest);

    world.insert_resource(MainRenderTarget {
        target: Some(main_render_target)
    });

    let glyph_material = load_material(
        ShaderSource::Glsl {
            vertex: GLYPH_VERTEX_SHADER,
            fragment: GLYPH_FRAGMENT_SHADER,
        },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("fg1", UniformType::Float4),
                UniformDesc::new("fg2", UniformType::Float4),
                UniformDesc::new("bg", UniformType::Float4),
                UniformDesc::new("outline", UniformType::Float4),
                UniformDesc::new("idx", UniformType::Int1),
            ],
            ..Default::default()
        },
    ).unwrap();

    let glyph_texture = load_texture("./src/cowboy.png").await.unwrap();

    world.insert_resource(GlyphMaterial {
        material: Some(glyph_material),
        texture: Some(glyph_texture),
    });

    rand::srand(miniquad::date::now() as u64);

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
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("iResolution", UniformType::Float2),
                UniformDesc::new("iTime", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .unwrap();

    loop {
        pref_size = get_preferred_size(texel_size);
        let pref_size_f32 = pref_size.as_vec2();
        
        let r = world.get_resource::<MainRenderTarget>().unwrap();
        let target = r.target.clone().unwrap();

        let cur_target_size = target.texture.size().as_ivec2();

        if cur_target_size != pref_size {
            main_render_target = render_target(pref_size.x as u32, pref_size.y as u32);
            main_render_target.texture.set_filter(FilterMode::Nearest);
        }

        set_camera(&Camera2D {
            zoom: vec2(1. / pref_size_f32.x * 2., 1. / pref_size_f32.y * 2.),
            target: vec2((pref_size_f32.x * 0.5f32).floor(), (pref_size_f32.y * 0.5f32).floor()),
            render_target: Some(target.clone()),
            ..Default::default()
        });
        clear_background(BLACK);

        starfield_material.set_uniform("iResolution", (pref_size_f32.x, pref_size_f32.y));
        starfield_material.set_uniform("direction_modifier", direction_modifier);
        gl_use_material(&starfield_material);
        draw_texture_ex(
            &target.texture,
            0.,
            0.,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(pref_size_f32.x, pref_size_f32.y)),
                ..Default::default()
            },
        );
        gl_use_default_material();

        schedule.run(&mut world);
        schedule_post_update.run(&mut world);

        set_default_camera();
        clear_background(ORANGE);
        crt_material.set_uniform("iTime", get_time() as f32);
        crt_material.set_uniform("iResolution", (pref_size_f32.x, pref_size_f32.y));
        gl_use_material(&crt_material);

        let screen_pad_x = (screen_width() - ((pref_size.x as f32) * (texel_size as f32))) * 0.5;
        let screen_pad_y = (screen_height() - ((pref_size.y as f32) * (texel_size as f32))) * 0.5;

        let dest_size = pref_size_f32 * vec2(texel_size as f32, texel_size as f32);


        draw_texture_ex(
            &target.texture,
            screen_pad_x,
            screen_pad_y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(dest_size),
                ..Default::default()
            },
        );
        gl_use_default_material();

        next_frame().await
    }
}