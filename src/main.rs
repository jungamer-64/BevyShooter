// src/main.rs
use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use bevy::asset::AssetMetaCheck;
use bevy::window::{WindowResolution, PrimaryWindow, WindowResized};
use bevy::ecs::entity::EntityHashSet;
use smallvec::SmallVec;
use std::time::Duration;
use std::ops::Range;
use core::fmt::Write as _;

// Debug-only logging macro
// - Requires BOTH "dev-logging" feature AND debug_assertions
// - Extra safety: even if feature is accidentally enabled in release, no output
#[cfg(all(feature = "dev-logging", debug_assertions))]
macro_rules! dlog {
    ($($t:tt)*) => { info!($($t)*) }
}

#[cfg(not(all(feature = "dev-logging", debug_assertions)))]
macro_rules! dlog {
    ($($t:tt)*) => {}
}

// Helper: map fastrand::f32() (0.0..1.0) to arbitrary Range<f32>
fn frand_range(range: Range<f32>) -> f32 {
    let t = fastrand::f32();
    range.start + t * (range.end - range.start)
}

// --- Constants ---
const PLAYER_SPEED: f32 = 500.0;
const BULLET_SPEED: f32 = 800.0;
const ENEMY_SPEED: f32 = 300.0;
const SPAWN_INTERVAL_SECONDS: f32 = 1.0;
const BULLET_SIZE: Vec2 = Vec2::new(10.0, 5.0);
const PLAYER_SIZE: Vec2 = Vec2::new(30.0, 30.0);
const ENEMY_SIZE: Vec2 = Vec2::new(30.0, 30.0);
const FIRE_COOLDOWN_SECONDS: f32 = 0.2;
const BULLET_LIFETIME_SECONDS: f32 = 3.0;
const PLAYER_MAX_HP: u32 = 3;
const INVINCIBILITY_SECONDS: f32 = 1.5;
const STAR_COUNT: usize = 50;
const STAR_SPEED_MIN: f32 = 50.0;
const STAR_SPEED_MAX: f32 = 200.0;
const EXPLOSION_DURATION: f32 = 0.5;
const ENEMY_BULLET_SPEED: f32 = 400.0;
const ENEMY_BULLET_SIZE: Vec2 = Vec2::new(8.0, 8.0);
const ENEMY_FIRE_INTERVAL: f32 = 2.0;
const DIFFICULTY_INTERVAL: f32 = 15.0;  // Level up every 15 seconds
const POWERUP_SIZE: Vec2 = Vec2::new(20.0, 20.0);
const POWERUP_DROP_RATE: f32 = 0.3;  // 30% drop rate
const POWERUP_SPEED: f32 = 100.0;    // Drift left speed
const POWERUP_TRIPLE_DURATION: f32 = 10.0;  // Triple shot duration
const POWERUP_RAPID_DURATION: f32 = 8.0;    // Rapid fire duration
const POWERUP_PIERCE_DURATION: f32 = 12.0;  // Pierce shot duration

// Z-layer constants for draw ordering (higher = closer to camera)
const Z_STARS: f32 = -10.0;
const Z_ENEMY: f32 = 0.0;
const Z_PLAYER: f32 = 1.0;
const Z_BULLET: f32 = 2.0;
const Z_POWERUP: f32 = 3.0;
const Z_FX: f32 = 5.0;

// Precomputed collision neighbor radii (avoids runtime ceil/div)
// Formula: ((max_a + max_b) * 0.5 / SpatialGrid::CELL_SIZE).ceil() + 1
// Note: All colliders use *0.5 scaling (half-extent style)
// Bullet collider = BULLET_SIZE*0.3 = ~3, Enemy collider = ENEMY_SIZE*0.5 = 15
const R_BULLET_ENEMY: i32 = 2;
// Player collider = PLAYER_SIZE*0.5 = 15, Enemy collider = ENEMY_SIZE*0.5 = 15
const R_PLAYER_ENEMY: i32 = 2;
// Player collider = PLAYER_SIZE*0.5 = 15, EnemyBullet collider = ENEMY_BULLET_SIZE = 8
const R_PLAYER_BULLET: i32 = 2;

fn main() {
    // Base window configuration
    #[allow(unused_mut)]
    let mut window = Window {
        title: "Bevy Shooter".into(),
        resolution: WindowResolution::new(800, 600),
        ..default()
    };

    // WASM-specific: bind to canvas, fit to parent, prevent browser shortcuts
    #[cfg(target_arch = "wasm32")]
    {
        window.canvas = Some("#bevy".to_string());
        window.fit_canvas_to_parent = true;
        window.prevent_default_event_handling = true;
    }

    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(window),
                ..default()
            })
            .set(AssetPlugin {
                // Required for WASM to avoid meta file lookups (server returns 404/HTML)
                #[cfg(target_arch = "wasm32")]
                meta_check: AssetMetaCheck::Never,
                ..default()
            })
        )
        .init_state::<GameState>()
        .init_state::<PlayState>()
        .init_resource::<SpawnState>()
        .init_resource::<Score>()
        .init_resource::<GameAssets>()
        .init_resource::<Difficulty>()
        .init_resource::<GameBounds>()
        .insert_resource(ClearColor(Color::srgb(0.02, 0.02, 0.05))) // Deep space
        .add_message::<ShakeEvent>()
        .add_systems(Startup, (setup_camera, load_assets, update_bounds, spawn_stars).chain())
        // PreUpdate: sync bounds from window resize events (WASM-friendly)
        .add_systems(PreUpdate, update_bounds_from_resize)
        .add_systems(PreUpdate, debug_asset_loading.run_if(|| cfg!(all(feature = "dev-logging", debug_assertions))))
        // Menu State
        .add_systems(OnEnter(GameState::Menu), setup_menu)
        .add_systems(Update, menu_input.run_if(in_state(GameState::Menu)))
        .add_systems(OnExit(GameState::Menu), cleanup_menu)
        // InGame State
        .add_systems(OnEnter(GameState::InGame), setup_game)
        .add_systems(
            Update,
            (
                // Phase 0: Tick timers FIRST (before they're consumed)
                update_fire_cooldown.before(player_shoot),
                update_powerups.before(player_shoot),
                update_difficulty.before(enemy_spawner),
                update_invincibility.before(collision_detection),
                // Phase 1: Input and spawning
                player_movement,
                enemy_spawner,
                enemy_fire_system,
                // Phase 2: Movement (after input)
                (bullet_movement, enemy_bullet_movement, enemy_movement, powerup_movement).after(player_movement),
                // Phase 2.5: Powerup collection (after movement, before shoot)
                powerup_collection.after(powerup_movement),
                // Phase 2.6: Player shoot (after powerup collection)
                player_shoot.after(powerup_collection),
                // Phase 3: Collision (after movement)
                collision_detection.after(bullet_movement).after(enemy_movement),
                // Phase 4: Updates and cleanup (after collision)
                scoreboard_update.after(collision_detection),
                update_powerup_ui.after(update_powerups),
                update_enemy_visuals.after(collision_detection),
                despawn_expired.after(collision_detection),
                // Phase 5: Camera shake (after collision)
                start_camera_shake.after(collision_detection),
                apply_camera_shake.after(start_camera_shake),
                update_explosions,
            ).run_if(in_state(GameState::InGame).and(in_state(PlayState::Playing))),
        )
        .add_systems(OnExit(GameState::InGame), (cleanup_game, reset_camera_shake))
        // PlayState Paused (orthogonal to GameState)
        .add_systems(OnEnter(PlayState::Paused), (setup_paused, reset_camera_shake))
        .add_systems(OnExit(PlayState::Paused), cleanup_paused)
        // GameOver State
        .add_systems(OnEnter(GameState::GameOver), setup_gameover)
        .add_systems(Update, gameover_input.run_if(in_state(GameState::GameOver)))
        .add_systems(OnExit(GameState::GameOver), cleanup_gameover)
        // Background stars (stop during pause for "true pause")
        .add_systems(Update, update_stars.run_if(in_state(PlayState::Playing).or(in_state(GameState::Menu)).or(in_state(GameState::GameOver))))
        .add_systems(Update, pause_input.run_if(in_state(GameState::InGame)))
        .run();
}

// --- Game State ---

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Menu,
    InGame,
    GameOver,
}

// PlayState is orthogonal to GameState (for pause without resetting)
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum PlayState {
    #[default]
    Playing,
    Paused,
}

// --- Components ---

#[derive(Component)]
struct Player;

#[derive(Component)]
struct Bullet;

#[derive(Component)]
struct Enemy;

#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider {
    size: Vec2,
}

#[derive(Component)]
struct ScoreText;

#[derive(Component)]
struct HpText;

#[derive(Component)]
struct MenuUI;

#[derive(Component)]
struct GameOverUI;

#[derive(Component)]
struct PausedUI;

#[derive(Component)]
struct PowerUpText;

#[derive(Component)]
struct GameEntity;

// New components for game improvements
#[derive(Component)]
struct FireCooldown(Timer);

#[derive(Component)]
struct Lifetime(Timer);

#[derive(Component)]
struct Health {
    current: u32,
    max: u32,
}

#[derive(Component)]
struct Invincible(Timer);

// Star background component
#[derive(Component)]
struct Star {
    speed: f32,
}

// Explosion effect component
#[derive(Component)]
struct Explosion(Timer);

// Camera shake components
#[derive(Component)]
struct MainCamera {
    base: Vec3,
}

#[derive(Component)]
struct CameraShake {
    timer: Timer,
    magnitude: f32,
}

// Camera shake message
#[derive(Message)]
struct ShakeEvent {
    magnitude: f32,
    duration: f32,
}

// Enemy bullet marker (to distinguish from player bullets)
#[derive(Component)]
struct EnemyBullet;

// Enemy fire timer (attached to each enemy)
#[derive(Component)]
struct EnemyFireTimer(Timer);

// Enemy type for different movement patterns
#[derive(Clone, Copy, PartialEq, Component)]
enum EnemyType {
    Normal,   // Moves straight left (HP: 1)
    Zigzag,   // Moves in a wave pattern (HP: 2)
    Chaser,   // Follows player Y position (HP: 3)
}

// Zigzag phase offset for visual variety
#[derive(Component)]
struct ZigzagPhase(f32);

impl EnemyType {
    fn initial_hp(&self) -> u32 {
        match self {
            EnemyType::Normal => 1,
            EnemyType::Zigzag => 2,
            EnemyType::Chaser => 3,
        }
    }
}

// Enemy health component (current/max for visual feedback)
#[derive(Component)]
struct EnemyHealth {
    current: u32,
    max: u32,
}

impl EnemyHealth {
    fn new(max: u32) -> Self {
        Self { current: max, max }
    }
}

// Enemy hit flash (brief white flash when damaged)
#[derive(Component)]
struct EnemyHitFlash(Timer);

// Power-up types
#[derive(Clone, Copy, PartialEq)]
enum PowerUpType {
    TripleShot,   // 3-way bullets
    RapidFire,    // Faster fire rate
    Shield,       // +1 HP
    PierceShot,   // Bullets pierce through enemies
}

// Power-up item component
#[derive(Component)]
struct PowerUpItem(PowerUpType);

// Pierce bullet marker (how many enemies it can pass through)
#[derive(Component)]
struct Pierce(u32);

// Track entities a pierce bullet has already hit (fixed-size, no heap)
#[derive(Component, Default)]
struct HitList {
    hit: [Option<Entity>; 3],
    len: u8,
}

impl HitList {
    fn contains(&self, e: Entity) -> bool {
        self.hit[..self.len as usize].iter().any(|&x| x == Some(e))
    }
    fn push(&mut self, e: Entity) {
        if (self.len as usize) < self.hit.len() {
            self.hit[self.len as usize] = Some(e);
            self.len += 1;
        }
    }
}

// Player stats for power-ups (with timers for temporary effects)
#[derive(Component)]
struct PlayerStats {
    weapon_level: u32,              // 0=normal, 1+=3-way
    fire_rate_bonus: f32,           // Cooldown reduction (0.0~0.5)
    triple_timer: Option<Timer>,    // Time remaining for triple shot
    rapid_timer: Option<Timer>,     // Time remaining for rapid fire
    pierce_timer: Option<Timer>,    // Time remaining for pierce shot
}

impl Default for PlayerStats {
    fn default() -> Self {
        Self {
            weapon_level: 0,
            fire_rate_bonus: 0.0,
            triple_timer: None,
            rapid_timer: None,
            pierce_timer: None,
        }
    }
}

// --- Resources ---

// Dynamic game bounds derived from Window size
#[derive(Resource)]
struct GameBounds {
    half_width: f32,
    half_height: f32,
}

impl Default for GameBounds {
    fn default() -> Self {
        Self { half_width: 400.0, half_height: 300.0 }
    }
}

impl GameBounds {
    fn player_x_range(&self, margin: f32) -> (f32, f32) {
        (-self.half_width + margin, self.half_width - margin)
    }
    fn player_y_range(&self, margin: f32) -> (f32, f32) {
        (-self.half_height + margin, self.half_height - margin)
    }
    fn spawn_x(&self) -> f32 { self.half_width + 50.0 }
    fn despawn_x(&self) -> f32 { -self.half_width - 50.0 }
    fn spawn_y_range(&self) -> Range<f32> {
        let min = -self.half_height + 50.0;
        let max = self.half_height - 50.0;
        // Safety: ensure range is valid even for tiny windows
        if min < max { min..max } else { 0.0..0.1 }
    }
}

#[derive(Resource)]
struct SpawnState {
    timer: Timer,
    current_interval: f32,
}

impl Default for SpawnState {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(SPAWN_INTERVAL_SECONDS, TimerMode::Repeating),
            current_interval: SPAWN_INTERVAL_SECONDS,
        }
    }
}

#[derive(Resource, Default)]
struct Score(u32);

// Asset cache resource
#[derive(Resource, Default)]
struct GameAssets {
    spaceship: Handle<Image>,
    bullet: Handle<Image>,
    asteroid: Handle<Image>,
}

// Difficulty resource
#[derive(Resource)]
struct Difficulty {
    level: u32,
    elapsed_time: f32,
}

impl Default for Difficulty {
    fn default() -> Self {
        Self { level: 0, elapsed_time: 0.0 }
    }
}

// Entry type for grid cells
type GridEntry = (Entity, Vec3, Vec2);

// Fixed-array spatial grid for zero-allocation collision detection
struct SpatialGrid {
    cells: Vec<SmallVec<[GridEntry; 4]>>,
    touched: Vec<usize>,
    cols: usize,
    rows: usize,
    offset_x: f32,
    offset_y: f32,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self {
            cells: Vec::new(),
            touched: Vec::new(),
            cols: 0,
            rows: 0,
            offset_x: 0.0,
            offset_y: 0.0,
        }
    }
}

impl SpatialGrid {
    const CELL_SIZE: f32 = 64.0;
    const INV_CELL_SIZE: f32 = 1.0 / 64.0;
    // Margin beyond screen bounds for spawn/despawn areas
    const MARGIN: f32 = 100.0;

    fn rebuild(&mut self, half_width: f32, half_height: f32) {
        // Calculate grid dimensions with margin for offscreen entities
        let total_width = (half_width + Self::MARGIN) * 2.0;
        let total_height = (half_height + Self::MARGIN) * 2.0;
        
        self.cols = (total_width * Self::INV_CELL_SIZE).ceil() as usize + 1;
        self.rows = (total_height * Self::INV_CELL_SIZE).ceil() as usize + 1;
        self.offset_x = half_width + Self::MARGIN;
        self.offset_y = half_height + Self::MARGIN;
        
        let total_cells = self.cols * self.rows;
        
        // Resize cells array, preserving existing SmallVecs where possible
        if self.cells.len() < total_cells {
            self.cells.resize_with(total_cells, SmallVec::new);
        }
        
        // Clear all cells
        for cell in &mut self.cells {
            cell.clear();
        }
        self.touched.clear();
        self.touched.reserve(64);
    }

    fn clear(&mut self) {
        for &idx in &self.touched {
            if idx < self.cells.len() {
                self.cells[idx].clear();
            }
        }
        self.touched.clear();
    }
    
    #[inline]
    fn cell_index(&self, pos: Vec3) -> Option<usize> {
        let cx = ((pos.x + self.offset_x) * Self::INV_CELL_SIZE).floor() as isize;
        let cy = ((pos.y + self.offset_y) * Self::INV_CELL_SIZE).floor() as isize;
        
        if cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows {
            Some((cy as usize) * self.cols + (cx as usize))
        } else {
            None
        }
    }
    
    #[inline]
    fn cell_coords(&self, pos: Vec3) -> (i32, i32) {
        (
            ((pos.x + self.offset_x) * Self::INV_CELL_SIZE).floor() as i32,
            ((pos.y + self.offset_y) * Self::INV_CELL_SIZE).floor() as i32,
        )
    }

    fn insert_center(&mut self, entity: Entity, pos: Vec3, size: Vec2) {
        if let Some(idx) = self.cell_index(pos) {
            let cell = &mut self.cells[idx];
            if cell.is_empty() {
                self.touched.push(idx);
            }
            cell.push((entity, pos, size));
        }
    }
    
    #[inline]
    fn get_cell(&self, cx: i32, cy: i32) -> Option<&[(Entity, Vec3, Vec2)]> {
        if cx >= 0 && cy >= 0 && (cx as usize) < self.cols && (cy as usize) < self.rows {
            let idx = (cy as usize) * self.cols + (cx as usize);
            Some(&self.cells[idx])
        } else {
            None
        }
    }
}

// Scratch space for collision detection (reused to avoid allocations)
#[derive(Default)]
struct CollisionScratch {
    hit_bullets: EntityHashSet,
    hit_enemies: EntityHashSet,
    grid: SpatialGrid,
    enemy_bullet_grid: SpatialGrid,
    reserved: bool,
    last_half_width: f32,
    last_half_height: f32,
}

// --- Setup Systems ---

fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        MainCamera { base: Vec3::ZERO },
    ));
}

fn load_assets(mut game_assets: ResMut<GameAssets>, asset_server: Res<AssetServer>) {
    game_assets.spaceship = asset_server.load("spaceship.webp");
    game_assets.bullet = asset_server.load("bullet.webp");
    game_assets.asteroid = asset_server.load("asteroid.webp");
}

// Debug system to check asset status
fn debug_asset_loading(
    asset_server: Res<AssetServer>,
    game_assets: Res<GameAssets>,
    mut diag_timer: Local<Timer>,
    time: Res<Time>,
) {
    // Initialize timer once (Local<Timer> starts with duration=ZERO)
    if diag_timer.duration() == Duration::ZERO {
        *diag_timer = Timer::from_seconds(1.0, TimerMode::Repeating);
    }

    diag_timer.tick(time.delta());
    if !diag_timer.just_finished() {
        return;
    }

    // Helper to print handle status
    let log_status = |name: &str, handle: &Handle<Image>| {
        if let Some(path) = handle.path() {
            dlog!("Asset {}: {:?} ({:?})", name, asset_server.load_state(handle), path);
        } else {
            dlog!("Asset {}: {:?} (No path)", name, asset_server.load_state(handle));
        }
    };

    log_status("Spaceship", &game_assets.spaceship);
    log_status("Bullet", &game_assets.bullet);
    log_status("Asteroid", &game_assets.asteroid);
}

fn update_bounds(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut bounds: ResMut<GameBounds>,
) {
    let Some(window) = windows.iter().next() else { return };
    bounds.half_width = window.width() * 0.5;
    bounds.half_height = window.height() * 0.5;
}

// Message-driven bounds update (avoids per-frame query in WASM)
fn update_bounds_from_resize(
    mut reader: MessageReader<WindowResized>,
    mut bounds: ResMut<GameBounds>,
) {
    for e in reader.read() {
        bounds.half_width = e.width * 0.5;
        bounds.half_height = e.height * 0.5;
    }
}

fn setup_menu(mut commands: Commands) {
    commands.spawn((
        Text::new("BEVY SHOOTER\n\nPress SPACE to Start"),
        TextFont {
            font_size: 50.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(20.0),
            left: Val::Percent(15.0),
            ..default()
        },
        MenuUI,
    ));
}

fn cleanup_menu(mut commands: Commands, query: Query<Entity, With<MenuUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn setup_game(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut spawn: ResMut<SpawnState>,
    mut difficulty: ResMut<Difficulty>,
    mut next_play_state: ResMut<NextState<PlayState>>,
    game_assets: Res<GameAssets>,
) {
    // Reset score, spawn state, difficulty, and ensure Playing state
    score.0 = 0;
    spawn.timer.reset();
    spawn.current_interval = SPAWN_INTERVAL_SECONDS;
    *difficulty = Difficulty::default();
    next_play_state.set(PlayState::Playing);

    // Player with HP and FireCooldown
    commands.spawn((
        Player,
        Sprite::from_image(game_assets.spaceship.clone()),
        Transform::from_xyz(-300.0, 0.0, Z_PLAYER).with_scale(Vec3::splat(0.5)),
        Collider { size: PLAYER_SIZE * 0.5 },
        Health { current: PLAYER_MAX_HP, max: PLAYER_MAX_HP },
        {
            // Start with cooldown finished so player can fire immediately
            let mut timer = Timer::from_seconds(FIRE_COOLDOWN_SECONDS, TimerMode::Once);
            timer.tick(Duration::from_secs_f32(FIRE_COOLDOWN_SECONDS));
            FireCooldown(timer)
        },
        PlayerStats::default(),
        GameEntity,
    ));

    // Scoreboard
    commands.spawn((
        Text::new("Score: 0"),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ScoreText,
        GameEntity,
    ));

    // HP display
    commands.spawn((
        Text::new(format!("HP: {}", PLAYER_MAX_HP)),
        TextFont {
            font_size: 30.0,
            ..default()
        },
        TextColor(Color::srgb(0.2, 1.0, 0.2)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            ..default()
        },
        HpText,
        GameEntity,
    ));

    // Power-up timer display
    commands.spawn((
        Text::new(""),
        TextFont {
            font_size: 24.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(45.0),
            right: Val::Px(10.0),
            ..default()
        },
        PowerUpText,
        GameEntity,
    ));
}

fn cleanup_game(mut commands: Commands, query: Query<Entity, With<GameEntity>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn setup_gameover(mut commands: Commands, score: Res<Score>) {
    commands.spawn((
        Text::new(format!("GAME OVER\n\nFinal Score: {}\n\nPress SPACE to Restart", score.0)),
        TextFont {
            font_size: 40.0,
            ..default()
        },
        TextColor(Color::srgb(1.0, 0.3, 0.3)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(30.0),
            left: Val::Percent(20.0),
            ..default()
        },
        GameOverUI,
    ));
}

fn cleanup_gameover(mut commands: Commands, query: Query<Entity, With<GameOverUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn setup_paused(mut commands: Commands) {
    commands.spawn((
        Text::new("PAUSED\n\nPress ESC to Resume"),
        TextFont {
            font_size: 50.0,
            ..default()
        },
        TextColor(Color::srgb(0.8, 0.8, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Percent(35.0),
            left: Val::Percent(30.0),
            ..default()
        },
        PausedUI,
    ));
}

fn cleanup_paused(mut commands: Commands, query: Query<Entity, With<PausedUI>>) {
    for entity in &query {
        commands.entity(entity).despawn();
    }
}

fn pause_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    play_state: Res<State<PlayState>>,
    mut next_play_state: ResMut<NextState<PlayState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        match play_state.get() {
            PlayState::Playing => next_play_state.set(PlayState::Paused),
            PlayState::Paused => next_play_state.set(PlayState::Playing),
        }
    }
}

// --- Input Systems ---

fn menu_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::InGame);
    }
}

fn gameover_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        next_state.set(GameState::InGame);
    }
}

// --- Gameplay Systems ---

fn player_movement(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    bounds: Res<GameBounds>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    let Some(mut transform) = query.iter_mut().next() else { return };
    
    let mut direction = Vec2::ZERO;
    if keyboard_input.pressed(KeyCode::ArrowUp) || keyboard_input.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowDown) || keyboard_input.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowLeft) || keyboard_input.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::ArrowRight) || keyboard_input.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }

    if direction.length() > 0.0 {
        direction = direction.normalize();
    }

    let (x_min, x_max) = bounds.player_x_range(20.0);
    let (y_min, y_max) = bounds.player_y_range(20.0);

    transform.translation.x = (transform.translation.x + direction.x * PLAYER_SPEED * time.delta_secs())
        .clamp(x_min, x_max);
    transform.translation.y = (transform.translation.y + direction.y * PLAYER_SPEED * time.delta_secs())
        .clamp(y_min, y_max);
}

fn update_fire_cooldown(time: Res<Time>, mut query: Query<&mut FireCooldown>) {
    for mut cooldown in &mut query {
        cooldown.0.tick(time.delta());
    }
}

fn player_shoot(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Transform, &mut FireCooldown, &PlayerStats), With<Player>>,
    game_assets: Res<GameAssets>,
) {
    if let Some((player_transform, mut cooldown, stats)) = query.iter_mut().next() {
        // Only fire if cooldown is finished
        if keyboard_input.pressed(KeyCode::Space) && cooldown.0.is_finished() {
            // Determine fire angles based on weapon level (fixed array, no heap)
            let mut angles = [0.0_f32; 3];
            let n = if stats.weapon_level >= 1 {
                angles[1] = 15.0_f32.to_radians();
                angles[2] = -15.0_f32.to_radians();
                3
            } else {
                1
            };
            
            // Check if pierce is active
            let has_pierce = stats.pierce_timer.is_some();
            
            for &angle in &angles[..n] {
                let direction = Vec2::new(angle.cos(), angle.sin()) * BULLET_SPEED;
                let p = player_transform.translation;
                let mut bullet = commands.spawn((
                    Bullet,
                    Sprite::from_image(game_assets.bullet.clone()),
                    Transform::from_xyz(p.x, p.y, Z_BULLET).with_scale(Vec3::splat(0.3)),
                    Velocity(direction),
                    Collider { size: BULLET_SIZE * 0.3 },
                    Lifetime(Timer::from_seconds(BULLET_LIFETIME_SECONDS, TimerMode::Once)),
                    GameEntity,
                ));
                
                // Add pierce if active (can hit 3 enemies)
                if has_pierce {
                    bullet.insert((Pierce(2), HitList::default())); // 2 extra pierces + hit tracking
                }
            }
            
            // Reset cooldown with fire rate bonus (capped at 90%)
            let bonus = stats.fire_rate_bonus.clamp(0.0, 0.9);
            let actual_cooldown = FIRE_COOLDOWN_SECONDS * (1.0 - bonus);
            cooldown.0.set_duration(Duration::from_secs_f32(actual_cooldown));
            cooldown.0.reset();
        }
    }
}

fn bullet_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<Bullet>>,
) {
    // Cap delta to prevent tunneling on tab recovery (WASM safety)
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

fn enemy_bullet_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Velocity), With<EnemyBullet>>,
) {
    // Cap delta to prevent tunneling on tab recovery (WASM safety)
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
    }
}

fn despawn_expired(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Lifetime)>,
) {
    for (entity, mut lifetime) in &mut query {
        lifetime.0.tick(time.delta());
        if lifetime.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_spawner(
    mut commands: Commands,
    time: Res<Time>,
    mut spawn: ResMut<SpawnState>,
    bounds: Res<GameBounds>,
    game_assets: Res<GameAssets>,
    difficulty: Res<Difficulty>,
) {
    // Update spawn interval only when difficulty changes (avoids mid-timer jitter)
    let new_interval = (SPAWN_INTERVAL_SECONDS - difficulty.level as f32 * 0.15).max(0.3);
    if (new_interval - spawn.current_interval).abs() > 0.01 {
        // Keep timer fraction to prevent spawn stutter
        let old_duration = spawn.timer.duration().as_secs_f32().max(0.0001);
        let fraction = (spawn.timer.elapsed_secs() / old_duration).clamp(0.0, 1.0);

        spawn.current_interval = new_interval;
        spawn.timer.set_duration(Duration::from_secs_f32(new_interval));
        spawn.timer.set_elapsed(Duration::from_secs_f32(fraction * new_interval));
        
        dlog!("Spawn interval changed to {:.2}s", new_interval);
    }
    
    spawn.timer.tick(time.delta());
    // Cap spawns per frame to prevent tab-recovery spikes (WASM safety)
    let spawn_count = spawn.timer.times_finished_this_tick().min(3);
    for _ in 0..spawn_count {
        let y = frand_range(bounds.spawn_y_range());
        
        // Adjust speed based on difficulty (max 2x)
        let speed_multiplier = (1.0 + difficulty.level as f32 * 0.1).min(2.0);
        
        // Adjust fire interval based on difficulty (min 0.5s)
        let fire_interval = (ENEMY_FIRE_INTERVAL - difficulty.level as f32 * 0.2).max(0.5);
        
        // Select enemy type based on difficulty
        let enemy_type = if difficulty.level >= 2 {
            match fastrand::u32(0..10) {
                0..=4 => EnemyType::Normal,   // 50%
                5..=7 => EnemyType::Zigzag,   // 30%
                _ => EnemyType::Chaser,        // 20%
            }
        } else {
            EnemyType::Normal
        };
        
        let mut enemy = commands.spawn((
            Enemy,
            enemy_type,
            EnemyHealth::new(enemy_type.initial_hp()),
            Sprite::from_image(game_assets.asteroid.clone()),
            Transform::from_xyz(bounds.spawn_x(), y, Z_ENEMY).with_scale(Vec3::splat(0.5)),
            Velocity(Vec2::new(-ENEMY_SPEED * speed_multiplier, 0.0)),
            Collider { size: ENEMY_SIZE * 0.5 },
            EnemyFireTimer(Timer::from_seconds(fire_interval, TimerMode::Repeating)),
            GameEntity,
        ));
        
        // Add phase offset for zigzag enemies
        if enemy_type == EnemyType::Zigzag {
            enemy.insert(ZigzagPhase(fastrand::f32() * std::f32::consts::TAU));
        }
    }
}

fn enemy_movement(
    mut commands: Commands,
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(Entity, &mut Transform, &mut Velocity, &EnemyType, Option<&ZigzagPhase>), With<Enemy>>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let player_y = player_query.iter().next().map(|t| t.translation.y).unwrap_or(0.0);
    let (y_min, y_max) = bounds.player_y_range(20.0);
    
    for (entity, mut transform, mut velocity, enemy_type, phase) in &mut query {
        // Apply type-specific movement
        match enemy_type {
            EnemyType::Normal => {
                // Normal: just move left (velocity.y stays 0)
            },
            EnemyType::Zigzag => {
                // Zigzag: oscillate up and down
                let offset = phase.map(|p| p.0).unwrap_or(0.0);
                velocity.0.y = (time.elapsed_secs() * 4.0 + transform.translation.x * 0.01 + offset).sin() * 150.0;
            },
            EnemyType::Chaser => {
                // Chaser: move towards player Y
                let diff = player_y - transform.translation.y;
                velocity.0.y = diff.clamp(-120.0, 120.0);
            },
        }
        
        // Cap delta to prevent warping on tab recovery (WASM safety)
        let dt = time.delta_secs().min(1.0 / 30.0);
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
        
        // Clamp Y position to bounds
        transform.translation.y = transform.translation.y.clamp(y_min, y_max);

        if transform.translation.x < bounds.despawn_x() {
            commands.entity(entity).despawn();
        }
    }
}

fn enemy_fire_system(
    mut commands: Commands,
    time: Res<Time>,
    mut enemy_query: Query<(&Transform, &mut EnemyFireTimer), With<Enemy>>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let player_pos = player_query.iter().next().map(|t| t.translation).unwrap_or(Vec3::ZERO);
    
    for (transform, mut fire_timer) in &mut enemy_query {
        fire_timer.0.tick(time.delta());
        // Cap fires per frame to prevent tab-recovery spikes
        let fire_count = fire_timer.0.times_finished_this_tick().min(2);
        for _ in 0..fire_count {
            // Calculate direction to player (aimed bullet)
            let delta = (player_pos - transform.translation).truncate();
            let direction = if delta.length_squared() > 1e-6 {
                delta.normalize()
            } else {
                Vec2::X * -1.0 // Default to left if overlapping
            };
            let velocity = direction * ENEMY_BULLET_SPEED;
            
            let ep = transform.translation;
            commands.spawn((
                EnemyBullet,
                Sprite::from_color(Color::srgb(1.0, 0.3, 0.3), ENEMY_BULLET_SIZE),
                Transform::from_xyz(ep.x, ep.y, Z_BULLET),
                Velocity(velocity),
                Collider { size: ENEMY_BULLET_SIZE },
                Lifetime(Timer::from_seconds(BULLET_LIFETIME_SECONDS, TimerMode::Once)),
                GameEntity,
            ));
        }
    }
}


fn update_difficulty(time: Res<Time>, mut difficulty: ResMut<Difficulty>) {
    difficulty.elapsed_time += time.delta_secs();
    let new_level = (difficulty.elapsed_time / DIFFICULTY_INTERVAL) as u32;
    if new_level > difficulty.level {
        difficulty.level = new_level;
        dlog!("Difficulty increased to level {}", difficulty.level);
    }
}

fn start_camera_shake(
    mut commands: Commands,
    mut ev: MessageReader<ShakeEvent>,
    mut q: Query<(Entity, &Transform, &mut MainCamera, Option<&mut CameraShake>)>,
) {
    let Some((entity, tf, mut cam, shake_opt)) = q.iter_mut().next() else { return; };

    // Aggregate shake events for this frame
    let mut mag = 0.0f32;
    let mut dur = 0.0f32;
    for e in ev.read() {
        mag = mag.max(e.magnitude);
        dur = dur.max(e.duration);
    }
    if mag <= 0.0 || dur <= 0.0 { return; }

    if let Some(mut shake) = shake_opt {
        // Update existing shake
        shake.magnitude = shake.magnitude.max(mag);
        let new_dur = shake.timer.duration().as_secs_f32().max(dur);
        shake.timer.set_duration(Duration::from_secs_f32(new_dur));
        shake.timer.reset();
    } else {
        // Start new shake
        cam.base = tf.translation;
        commands.entity(entity).insert(CameraShake {
            timer: Timer::from_seconds(dur, TimerMode::Once),
            magnitude: mag,
        });
    }
}

fn apply_camera_shake(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut Transform, &MainCamera, &mut CameraShake)>,
) {
    for (entity, mut tf, cam, mut shake) in &mut q {
        shake.timer.tick(time.delta());

        let dur = shake.timer.duration().as_secs_f32().max(0.0001);
        let t = 1.0 - (shake.timer.elapsed_secs() / dur).clamp(0.0, 1.0);

        let dx = frand_range(-1.0..1.0) * shake.magnitude * t;
        let dy = frand_range(-1.0..1.0) * shake.magnitude * t;

        tf.translation = cam.base + Vec3::new(dx, dy, 0.0);

        if shake.timer.is_finished() {
            tf.translation = cam.base;
            commands.entity(entity).remove::<CameraShake>();
        }
    }
}

fn reset_camera_shake(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Transform, &MainCamera, &CameraShake)>,
) {
    for (entity, mut tf, cam, _) in &mut q {
        tf.translation = cam.base;
        commands.entity(entity).remove::<CameraShake>();
    }
}

fn update_invincibility(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Invincible, &mut Sprite)>,
) {
    for (entity, mut invincible, mut sprite) in &mut query {
        invincible.0.tick(time.delta());
        
        // Flash effect: toggle visibility based on time
        let elapsed = invincible.0.elapsed_secs();
        let visible = (elapsed * 10.0) as i32 % 2 == 0;
        sprite.color = if visible {
            Color::WHITE
        } else {
            Color::srgba(1.0, 1.0, 1.0, 0.3)
        };
        
        if invincible.0.is_finished() {
            // Remove invincibility and restore color
            sprite.color = Color::WHITE;
            commands.entity(entity).remove::<Invincible>();
        }
    }
}

fn collision_detection(
    mut commands: Commands,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<GameState>>,
    mut scratch: Local<CollisionScratch>,
    mut shake: MessageWriter<ShakeEvent>,
    bounds: Res<GameBounds>,
    mut bullet_query: Query<(Entity, &Transform, &Collider, Option<&mut Pierce>, Option<&mut HitList>), With<Bullet>>,
    mut enemy_hp_query: Query<(&mut EnemyHealth, &EnemyType), With<Enemy>>,
    enemy_ro_query: Query<(Entity, &Transform, &Collider), With<Enemy>>,
    enemy_bullet_query: Query<(Entity, &Transform, &Collider), With<EnemyBullet>>,
    mut player_query: Query<(Entity, &Transform, &Collider, &mut Health, Option<&Invincible>), With<Player>>,
) {
    let scratch = &mut *scratch;
    
    // Debug-only validation that R_* constants are still correct for current sizes
    #[cfg(debug_assertions)]
    {
        let cell = SpatialGrid::CELL_SIZE;
        // Bullet vs Enemy
        let r1 = ((BULLET_SIZE.x.max(BULLET_SIZE.y)*0.3 + ENEMY_SIZE.x.max(ENEMY_SIZE.y)*0.5) * 0.5 / cell).ceil() as i32 + 1;
        debug_assert!(R_BULLET_ENEMY >= r1, "R_BULLET_ENEMY outdated: need {}", r1);
        // Player vs Enemy
        let r2 = ((PLAYER_SIZE.x.max(PLAYER_SIZE.y)*0.5 + ENEMY_SIZE.x.max(ENEMY_SIZE.y)*0.5) * 0.5 / cell).ceil() as i32 + 1;
        debug_assert!(R_PLAYER_ENEMY >= r2, "R_PLAYER_ENEMY outdated: need {}", r2);
        // Player vs EnemyBullet
        let r3 = ((PLAYER_SIZE.x.max(PLAYER_SIZE.y)*0.5 + ENEMY_BULLET_SIZE.x.max(ENEMY_BULLET_SIZE.y)) * 0.5 / cell).ceil() as i32 + 1;
        debug_assert!(R_PLAYER_BULLET >= r3, "R_PLAYER_BULLET outdated: need {}", r3);
    }
    
    // Detect resize and rebuild grid if needed
    let resized = 
        (scratch.last_half_width - bounds.half_width).abs() > 0.5 ||
        (scratch.last_half_height - bounds.half_height).abs() > 0.5;

    if scratch.grid.cols == 0 || resized {
        scratch.grid.rebuild(bounds.half_width, bounds.half_height);
        scratch.enemy_bullet_grid.rebuild(bounds.half_width, bounds.half_height);
        scratch.last_half_width = bounds.half_width;
        scratch.last_half_height = bounds.half_height;
    } else {
        // Only clear if not rebuilt (rebuild already clears)
        scratch.grid.clear();
        scratch.enemy_bullet_grid.clear();
    }

    if !scratch.reserved {
        scratch.hit_bullets.reserve(128);
        scratch.hit_enemies.reserve(64);
        scratch.reserved = true;
    }

    // Clear scratch sets
    scratch.hit_bullets.clear();
    scratch.hit_enemies.clear();

    // Populate grid with enemies (center cell only + snapshot pos/size)
    for (entity, transform, collider) in &enemy_ro_query {
        scratch.grid.insert_center(entity, transform.translation, collider.size);
    }

    // Populate grid with enemy bullets
    for (entity, transform, collider) in &enemy_bullet_query {
        scratch.enemy_bullet_grid.insert_center(entity, transform.translation, collider.size);
    }

    // --- Bullet vs Enemy (scoped to isolate borrows) ---
    // Local shake aggregation to reduce message buffer writes
    let mut shake_mag = 0.0f32;
    let mut shake_dur = 0.0f32;
    macro_rules! add_shake {
        ($m:expr, $d:expr) => {{
            shake_mag = shake_mag.max($m);
            shake_dur = shake_dur.max($d);
        }};
    }
    {
        let CollisionScratch { hit_bullets, hit_enemies, grid, .. } = &mut *scratch;

    // Bullet vs Enemy (Optimized: 3x3 Neighbor Query, No Allocations)
    'bullet_loop: for (bullet_entity, bullet_transform, bullet_collider, mut pierce_opt, mut hitlist_opt) in &mut bullet_query {
        if hit_bullets.contains(&bullet_entity) { continue; }

        let b_pos = bullet_transform.translation;
        let b_size = bullet_collider.size;
        let (cx, cy) = grid.cell_coords(b_pos);

        // Use precomputed const radius for bullet vs enemy
        let r = R_BULLET_ENEMY;

        // Check neighbors around the bullet's cell
        'neighbor_loop: for x in (cx - r)..=(cx + r) {
            for y in (cy - r)..=(cy + r) {
                if let Some(entries) = grid.get_cell(x, y) {
                    for &(enemy_entity, enemy_pos, enemy_size) in entries {
                        // Avoid hitting already destroyed enemies
                        if hit_enemies.contains(&enemy_entity) { continue; }
                        
                        // Skip if pierce bullet already hit this enemy
                        if let Some(ref hits) = hitlist_opt {
                            if hits.contains(enemy_entity) { continue; }
                        }

                        // Check collision using snapshot data (no ECS access yet)
                        if collide(b_pos, b_size, enemy_pos, enemy_size) {
                            // Only NOW fetch mutable component for HP modification
                            if let Ok((mut enemy_hp, enemy_type)) = enemy_hp_query.get_mut(enemy_entity) {
                                // Reduce enemy HP
                                enemy_hp.current = enemy_hp.current.saturating_sub(1);
                                
                                if enemy_hp.current == 0 {
                                    // Enemy destroyed
                                    hit_enemies.insert(enemy_entity);
                                    spawn_explosion(&mut commands, enemy_pos);
                                    
                                    if fastrand::f32() < POWERUP_DROP_RATE {
                                        let power_type = match fastrand::u32(0..4) {
                                            0 => PowerUpType::TripleShot,
                                            1 => PowerUpType::RapidFire,
                                            2 => PowerUpType::PierceShot,
                                            _ => PowerUpType::Shield,
                                        };
                                        spawn_powerup(&mut commands, enemy_pos, power_type);
                                    }
                                    
                                    commands.entity(enemy_entity).despawn();
                                    
                                    let points = match enemy_type {
                                        EnemyType::Normal => 10,
                                        EnemyType::Zigzag => 20,
                                        EnemyType::Chaser => 30,
                                    };
                                    score.0 += points;
                                    add_shake!(4.0, 0.06);
                                } else {
                                    // Enemy hit
                                    commands.entity(enemy_entity).insert(
                                        EnemyHitFlash(Timer::from_seconds(0.06, TimerMode::Once))
                                    );
                                    add_shake!(2.0, 0.03);
                                }
                                
                                // Handle pierce
                                if let Some(ref mut pierce) = pierce_opt {
                                    if let Some(ref mut hits) = hitlist_opt {
                                        hits.push(enemy_entity);
                                    }
                                    if pierce.0 > 0 {
                                        pierce.0 -= 1;
                                        continue 'bullet_loop;
                                    }
                                }
                                
                                hit_bullets.insert(bullet_entity);
                                commands.entity(bullet_entity).despawn();
                                break 'neighbor_loop;
                            }
                        }
                    }
                }
            }
        }
    }
    } // End of bullet vs enemy scope (borrows released)

    // Enemy vs Player and EnemyBullet vs Player
    if let Some((player_entity, player_transform, player_collider, mut health, invincible)) = player_query.iter_mut().next() {
        let invincible_active = invincible.is_some();

        if invincible_active {
            // Invincibility: consume overlapping enemy bullets (grid-optimized)
            let p_pos = player_transform.translation;
            let p_size = player_collider.size;
            let (bx, by) = scratch.enemy_bullet_grid.cell_coords(p_pos);
            // Use precomputed const radius for player vs enemy bullet
            let r_bullet = R_PLAYER_BULLET;
            
            for x in (bx - r_bullet)..=(bx + r_bullet) {
                for y in (by - r_bullet)..=(by + r_bullet) {
                    if let Some(entries) = scratch.enemy_bullet_grid.get_cell(x, y) {
                        for &(bullet_entity, bullet_pos, bullet_size) in entries {
                            if collide(p_pos, p_size, bullet_pos, bullet_size) {
                                commands.entity(bullet_entity).despawn();
                            }
                        }
                    }
                }
            }
        }

        // Shared logic for player collision (Deduplicated)
        let p_pos = player_transform.translation;
        let p_size = player_collider.size;
        let (cx, cy) = scratch.grid.cell_coords(p_pos);
        // Use precomputed const radius for player vs enemy
        let r = R_PLAYER_ENEMY;

        // Decompose grid to avoid capture conflict
        let grid = &scratch.grid; 

        if invincible_active {
             // Allow ramming enemies while invincible (inlined for performance)
             for x in (cx - r)..=(cx + r) {
                 for y in (cy - r)..=(cy + r) {
                     if let Some(entries) = grid.get_cell(x, y) {
                         for &(enemy_entity, enemy_pos, enemy_size) in entries {
                             if scratch.hit_enemies.contains(&enemy_entity) { continue; }
                             
                             if collide(p_pos, p_size, enemy_pos, enemy_size) {
                                 spawn_explosion(&mut commands, enemy_pos);
                                 commands.entity(enemy_entity).despawn();
                                 score.0 += 5;
                                 add_shake!(4.0, 0.06);
                                 scratch.hit_enemies.insert(enemy_entity);
                             }
                         }
                     }
                 }
             }
             
             // Write aggregated shake before returning
             if shake_mag > 0.0 && shake_dur > 0.0 {
                 shake.write(ShakeEvent { magnitude: shake_mag, duration: shake_dur });
             }
             return;
        } else {
            // Check enemy collision (inlined for performance)
            'col_loop: for x in (cx - r)..=(cx + r) {
                for y in (cy - r)..=(cy + r) {
                    if let Some(entries) = grid.get_cell(x, y) {
                        for &(enemy_entity, enemy_pos, enemy_size) in entries {
                            if scratch.hit_enemies.contains(&enemy_entity) { continue; }
                            
                            if collide(p_pos, p_size, enemy_pos, enemy_size) {
                                // Reduce HP
                                health.current = health.current.saturating_sub(1);
                                
                                commands.entity(enemy_entity).despawn();
                                scratch.hit_enemies.insert(enemy_entity);
                                
                                if health.current == 0 {
                                    next_state.set(GameState::GameOver);
                                    add_shake!(20.0, 0.3);
                                } else {
                                    commands.entity(player_entity).insert(
                                        Invincible(Timer::from_seconds(INVINCIBILITY_SECONDS, TimerMode::Once))
                                    );
                                    add_shake!(14.0, 0.15);
                                }
                                // Flush before return
                                if shake_mag > 0.0 && shake_dur > 0.0 {
                                    shake.write(ShakeEvent { magnitude: shake_mag, duration: shake_dur });
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
        
        // Check enemy bullet collision (grid-optimized)
        let p_pos = player_transform.translation;
        let p_size = player_collider.size;
        let (bx, by) = scratch.enemy_bullet_grid.cell_coords(p_pos);
        // Use precomputed const radius for player vs enemy bullet
        let r_bullet = R_PLAYER_BULLET;
        
        for x in (bx - r_bullet)..=(bx + r_bullet) {
            for y in (by - r_bullet)..=(by + r_bullet) {
                if let Some(entries) = scratch.enemy_bullet_grid.get_cell(x, y) {
                    for &(bullet_entity, bullet_pos, bullet_size) in entries {
                        if collide(p_pos, p_size, bullet_pos, bullet_size) {
                            // Reduce HP
                            health.current = health.current.saturating_sub(1);
                            
                            // Destroy the bullet
                            commands.entity(bullet_entity).despawn();
                            
                            if health.current == 0 {
                                // Game Over
                                next_state.set(GameState::GameOver);
                                add_shake!(20.0, 0.3);
                            } else {
                                // Add invincibility and big shake
                                commands.entity(player_entity).insert(
                                    Invincible(Timer::from_seconds(INVINCIBILITY_SECONDS, TimerMode::Once))
                                );
                                add_shake!(14.0, 0.15);
                            }
                            // Flush before return
                            if shake_mag > 0.0 && shake_dur > 0.0 {
                                shake.write(ShakeEvent { magnitude: shake_mag, duration: shake_dur });
                            }
                            return;
                        }
                    }
                }
            }
        }
    }

    // Write aggregated shake (only if no early return happened)
    if shake_mag > 0.0 && shake_dur > 0.0 {
        shake.write(ShakeEvent { magnitude: shake_mag, duration: shake_dur });
    }
}

fn collide(pos_a: Vec3, size_a: Vec2, pos_b: Vec3, size_b: Vec2) -> bool {
    let a_min = pos_a.truncate() - size_a / 2.0;
    let a_max = pos_a.truncate() + size_a / 2.0;
    let b_min = pos_b.truncate() - size_b / 2.0;
    let b_max = pos_b.truncate() + size_b / 2.0;

    a_min.x < b_max.x && a_max.x > b_min.x && a_min.y < b_max.y && a_max.y > b_min.y
}

fn scoreboard_update(
    score: Res<Score>,
    mut score_query: Query<&mut Text, (With<ScoreText>, Without<HpText>)>,
    player_query: Query<Ref<Health>, With<Player>>,
    mut hp_query: Query<&mut Text, (With<HpText>, Without<ScoreText>)>,
) {
    // Update score only if changed
    if score.is_changed() {
        for mut text in &mut score_query {
            text.0.clear();
            let _ = write!(text.0, "Score: {}", score.0);
        }
    }
    
    // Update HP only if changed
    if let Some(health) = player_query.iter().next() {
        if health.is_changed() {
            for mut text in &mut hp_query {
                text.0.clear();
                let _ = write!(text.0, "HP: {}", health.current);
            }
        }
    }
}

fn update_powerup_ui(
    time: Res<Time>,
    player_query: Query<&PlayerStats, With<Player>>,
    mut text_query: Query<&mut Text, (With<PowerUpText>, Without<ScoreText>, Without<HpText>)>,
    mut ui_timer: Local<Timer>,
) {
    if ui_timer.duration() == Duration::ZERO {
        *ui_timer = Timer::from_seconds(0.1, TimerMode::Repeating);
    }
    ui_timer.tick(time.delta());
    if !ui_timer.just_finished() {
        return;
    }

    let Some(stats) = player_query.iter().next() else { return; };
    let Some(mut text) = text_query.iter_mut().next() else { return; };
    
    // reuse internal string buffer to minimize allocations
    text.0.clear();
    let mut first = true;
    
    // Helper closure correctly captures variables and updates text
    let mut push_line = |label: &str, timer: &Timer| {
        let remaining = (timer.duration().as_secs_f32() - timer.elapsed_secs()).max(0.0);
        if remaining > 0.0 {
            if !first {
                text.0.push('\n');
            }
            // Use core::fmt::Write to write directly to String
            // We ignore errors since writing to String usually succeeds
            let _ = write!(text.0, "{}: {:.1}s", label, remaining);
            first = false;
        }
    };

    if let Some(ref timer) = stats.triple_timer {
        push_line("TRIPLE", timer);
    }
    if let Some(ref timer) = stats.rapid_timer {
        push_line("RAPID", timer);
    }
    if let Some(ref timer) = stats.pierce_timer {
        push_line("PIERCE", timer);
    }
}

// Update enemy visuals: hit flash (white) and HP-based color (red when damaged)
fn update_enemy_visuals(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, Ref<EnemyHealth>, Option<&mut EnemyHitFlash>, &mut Sprite), With<Enemy>>,
) {
    for (e, hp, flash_opt, mut sprite) in &mut q {
        let mut should_update = hp.is_changed();

        // Flash takes priority (brief white when hit)
        if let Some(mut flash) = flash_opt {
            flash.0.tick(time.delta());
            if !flash.0.is_finished() {
                sprite.color = Color::WHITE;
                continue;
            }
            commands.entity(e).remove::<EnemyHitFlash>();
            should_update = true; // Force update to restore color after flash
        }

        // HP ratio determines color: full HP = white, damaged = reddish
        if should_update {
            let t = (hp.current as f32 / hp.max.max(1) as f32).clamp(0.0, 1.0);
            sprite.color = Color::srgb(1.0, t, t);
        }
    }
}

// --- Star Background Systems ---

fn spawn_stars(mut commands: Commands, bounds: Res<GameBounds>) {
    for _ in 0..STAR_COUNT {
        let x = frand_range(-bounds.half_width..bounds.half_width);
        let y = frand_range(-bounds.half_height..bounds.half_height);
        let speed = frand_range(STAR_SPEED_MIN..STAR_SPEED_MAX);
        let size = frand_range(1.0..4.0);
        let brightness = frand_range(0.3..1.0);
        
        commands.spawn((
            Sprite::from_color(
                Color::srgba(brightness, brightness, brightness, 1.0),
                Vec2::new(size, size),
            ),
            Transform::from_xyz(x, y, Z_STARS), // Behind everything
            Star { speed },
        ));
    }
}

fn update_stars(
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(&mut Transform, &Star)>,
) {
    // Cap delta to prevent warping on tab recovery (WASM safety)
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (mut transform, star) in &mut query {
        // Move star left
        transform.translation.x -= star.speed * dt;
        
        // Wrap around when off-screen
        if transform.translation.x < bounds.despawn_x() {
            transform.translation.x = bounds.spawn_x();
            // Randomize y position when wrapping
            transform.translation.y = frand_range(-bounds.half_height..bounds.half_height);
        }
    }
}

// --- Explosion Systems ---

fn spawn_explosion(commands: &mut Commands, position: Vec3) {
    for _ in 0..12 {
        let angle = frand_range(0.0..std::f32::consts::TAU);
        let speed = frand_range(100.0..300.0);
        let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);
        let size = frand_range(3.0..8.0);
        let g = frand_range(0.3..0.8);

        commands.spawn((
            Sprite::from_color(
                Color::srgb(1.0, g, 0.0),
                Vec2::new(size, size),
            ),
            Transform::from_xyz(position.x, position.y, Z_FX),
            Velocity(velocity),
            Explosion(Timer::from_seconds(EXPLOSION_DURATION, TimerMode::Once)),
            GameEntity,
        ));
    }
}

fn update_explosions(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &Velocity, &mut Explosion, &mut Sprite)>,
) {
    for (entity, mut transform, velocity, mut explosion, mut sprite) in &mut query {
        // Move particle
        transform.translation.x += velocity.0.x * time.delta_secs();
        transform.translation.y += velocity.0.y * time.delta_secs();
        
        // Tick timer
        explosion.0.tick(time.delta());
        
        // Fade out
        let alpha = (1.0 - explosion.0.elapsed_secs() / EXPLOSION_DURATION).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_alpha(alpha);
        
        // Despawn when finished
        if explosion.0.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// --- Power-Up Systems ---

fn spawn_powerup(commands: &mut Commands, position: Vec3, power_type: PowerUpType) {
    let color = match power_type {
        PowerUpType::TripleShot => Color::srgb(0.2, 0.6, 1.0),  // Blue
        PowerUpType::RapidFire => Color::srgb(1.0, 1.0, 0.2),   // Yellow
        PowerUpType::PierceShot => Color::srgb(0.8, 0.2, 1.0),  // Purple
        PowerUpType::Shield => Color::srgb(0.2, 1.0, 0.2),      // Green
    };
    
    commands.spawn((
        PowerUpItem(power_type),
        Sprite::from_color(color, POWERUP_SIZE),
        Transform::from_xyz(position.x, position.y, Z_POWERUP),
        Velocity(Vec2::new(-POWERUP_SPEED, 0.0)),
        Collider { size: POWERUP_SIZE },
        GameEntity,
    ));
}

fn powerup_movement(
    mut commands: Commands,
    time: Res<Time>,
    bounds: Res<GameBounds>,
    mut query: Query<(Entity, &mut Transform, &Velocity), With<PowerUpItem>>,
) {
    // Cap delta to prevent warping on tab recovery (WASM safety)
    let dt = time.delta_secs().min(1.0 / 30.0);
    for (entity, mut transform, velocity) in &mut query {
        transform.translation.x += velocity.0.x * dt;
        transform.translation.y += velocity.0.y * dt;
        
        // Despawn if off screen
        if transform.translation.x < bounds.despawn_x() {
            commands.entity(entity).despawn();
        }
    }
}

fn powerup_collection(
    mut commands: Commands,
    powerup_query: Query<(Entity, &Transform, &Collider, &PowerUpItem)>,
    mut player_query: Query<(&Transform, &Collider, &mut PlayerStats, &mut Health), With<Player>>,
) {
    if let Ok((player_tf, player_col, mut stats, mut health)) = player_query.single_mut() {
        for (entity, tf, col, item) in &powerup_query {
            if collide(player_tf.translation, player_col.size, tf.translation, col.size) {
                match item.0 {
                    PowerUpType::TripleShot => {
                        stats.weapon_level = 1;
                        // Start or reset timer
                        stats.triple_timer = Some(Timer::from_seconds(POWERUP_TRIPLE_DURATION, TimerMode::Once));
                        dlog!("Power-up: Triple Shot! ({:.0}s)", POWERUP_TRIPLE_DURATION);
                    },
                    PowerUpType::RapidFire => {
                        stats.fire_rate_bonus = 0.4; // Fixed bonus instead of stacking
                        // Start or reset timer
                        stats.rapid_timer = Some(Timer::from_seconds(POWERUP_RAPID_DURATION, TimerMode::Once));
                        dlog!("Power-up: Rapid Fire! ({:.0}s)", POWERUP_RAPID_DURATION);
                    },
                    PowerUpType::PierceShot => {
                        // Start or reset timer
                        stats.pierce_timer = Some(Timer::from_seconds(POWERUP_PIERCE_DURATION, TimerMode::Once));
                        dlog!("Power-up: Pierce Shot! ({:.0}s)", POWERUP_PIERCE_DURATION);
                    },
                    PowerUpType::Shield => {
                        health.current = (health.current + 1).min(health.max);
                        dlog!("Power-up: Shield! (HP: {})", health.current);
                    },
                }
                commands.entity(entity).despawn();
            }
        }
    }
}


fn update_powerups(
    time: Res<Time>,
    mut query: Query<&mut PlayerStats, With<Player>>,
) {
    for mut stats in &mut query {
        // Update triple shot timer
        if let Some(ref mut timer) = stats.triple_timer {
            timer.tick(time.delta());
            if timer.is_finished() {
                stats.weapon_level = 0;
                stats.triple_timer = None;
                dlog!("Triple Shot expired!");
            }
        }
        
        // Update rapid fire timer
        if let Some(ref mut timer) = stats.rapid_timer {
            timer.tick(time.delta());
            if timer.is_finished() {
                stats.fire_rate_bonus = 0.0;
                stats.rapid_timer = None;
                dlog!("Rapid Fire expired!");
            }
        }
        
        // Update pierce shot timer
        if let Some(ref mut timer) = stats.pierce_timer {
            timer.tick(time.delta());
            if timer.is_finished() {
                stats.pierce_timer = None;
                dlog!("Pierce Shot expired!");
            }
        }
    }
}
