use bevy::{
    prelude::*,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};

fn main() {
    App::build()
        .add_default_plugins()
        .add_resource(SoftDropTimer(Timer::from_seconds(0.750)))
        .add_resource(PrintInfoTimer(Timer::from_seconds(1.0)))
        .add_startup_system(setup.system())
        .add_system(print_info.system())
        .add_system(move_current_tetromino.system())
        .add_system(rotate_current_tetromino.system())
        .add_system(keep_current_tetromino_in_bounds.system())
        .add_system(add_current_to_heap_and_spawn_new.system())
        .run();
}

struct SoftDropTimer(Timer);

struct PrintInfoTimer(Timer);

// Base entity, everything is made out of blocks
struct Block {
    color: Color,
}

struct Matrix;

// A block can be positioned on the matrix, either as part of the current
// tetromino or as part of the heap.
struct MatrixPosition {
    x: i32,
    y: i32,
}

// A block can be part of a tetromino. Stores the block's index within that
// tetromino for the purpose of rotation.
struct Tetromino {
    tetromino_type: TetrominoType,
    index: MatrixPosition,
}

// A block can be part of the currently controlled tetromino.
struct CurrentTetromino;

// A block can be part of the currently held tetromino.
struct HeldTetromino;

// Tracks whether a tetromino was already held since the last tetromino was
// added to the heap.
struct AlreadyHeld(bool);

// A block can be part of one of the tetrominos that are next in line. Stores
// the position of that tetromino in line.
struct NextTetromino {
    pos_in_line: u8,
}

// A block can be part of the heap.
struct Heap;

impl Block {
    const SIZE: f32 = 25.0;
}

#[derive(Copy, Clone)]
enum TetrominoType {
    Line    = 0,
    Square  = 1,
    T       = 2,
    S       = 3,
    Z       = 4,
    L       = 5,
    J       = 6,
}

impl Tetromino {
    const BLOCK_INDICES: [[(i32, i32); 4]; 7] = [
        [ // line, cyan
            (1, 3),
            (1, 2),
            (1, 1),
            (1, 0),
        ],
        [ // square, yellow
            (1, 1),
            (1, 2),
            (2, 1),
            (2, 2),
        ],
        [ // T, purple
            (0, 1),
            (1, 1),
            (2, 1),
            (1, 2),
        ],
        [ // Z, red
            (0, 2),
            (1, 2),
            (1, 1),
            (2, 1),
        ],
        [ // S, green
            (2, 2),
            (1, 2),
            (1, 1),
            (0, 1),
        ],
        [ // L, blue
            (0, 2),
            (0, 1),
            (1, 1),
            (2, 1),
        ],
        [ // J, orange
            (0, 1),
            (1, 1),
            (2, 1),
            (2, 2),
        ],
    ];

    const COLORS: [(f32, f32, f32); 7] = [
        (0.0, 0.7, 0.7), // line, cyan
        (0.7, 0.7, 0.0), // square, yellow
        (0.7, 0.0, 0.7), // T, purple
        (0.7, 0.0, 0.0), // Z, red
        (0.0, 0.7, 0.0), // S, green
        (0.0, 0.0, 0.7), // L, blue
        (0.9, 0.25, 0.0), // J, orange
    ];

    const SIZES: [i32; 7] = [
        4, // line, cyan
        4, // square, yellow
        3, // T, purple
        3, // Z, red
        3, // S, green
        3, // L, blue
        3, // J, orange
    ];

    fn blocks_from_type(tetromino_type: TetrominoType) -> Vec<(Block, Tetromino)> {
        let type_usize = tetromino_type as usize;
        let color = Tetromino::COLORS[type_usize];

        Tetromino::BLOCK_INDICES[type_usize].iter()
            .map(|index| {
                (
                    Block {
                        color: Color::rgb(color.0, color.1, color.2),
                    },
                    Tetromino {
                        index: MatrixPosition { x: index.0, y: index.1 },
                        tetromino_type
                    }
                )
            })
            .collect()
    }
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    commands
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
        .spawn(SpriteComponents {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite {
                size: Vec2::new(Block::SIZE * 10.0, Block::SIZE * 22.0),
            },
            ..Default::default()
        })
        .with(Matrix)
    ;

    spawn_current_tetromino(commands, materials);
}

fn print_info(
    time: Res<Time>,
    mut timer: ResMut<PrintInfoTimer>,
    mut matrix_query: Query<(&Matrix, &Sprite, &Translation)>,
    mut current_query: Query<(&CurrentTetromino, &Translation)>
) {
    timer.0.tick(time.delta_seconds);

    if timer.0.finished {
        for matrix in &mut matrix_query.iter() {
            println!("Matrix size: {:?}", matrix.1.size);
            println!("Matrix translation: {:?}", matrix.2);
        }

        for current in &mut current_query.iter() {
            println!("Current translation: {:?}", current.1);
        }
        timer.0.reset();
    }
}

fn move_current_tetromino(
    time: Res<Time>,
    mut soft_drop_timer: ResMut<SoftDropTimer>,
    keyboard_input: Res<Input<KeyCode>>,
    mut current_query: Query<(&mut Tetromino, &CurrentTetromino, &mut Translation)>
) {
    soft_drop_timer.0.tick(time.delta_seconds);

    let mut move_x = 0.0;
    let mut move_y = 0.0;
    if keyboard_input.just_pressed(KeyCode::J) {
        move_x -= Block::SIZE;
    }

    if keyboard_input.just_pressed(KeyCode::L) {
        move_x += Block::SIZE;
    }

    if keyboard_input.just_pressed(KeyCode::K) || soft_drop_timer.0.finished {
        move_y -= Block::SIZE;
        soft_drop_timer.0.reset();
    }

    for (_tetromino, _current, mut translation) in &mut current_query.iter() {
        *translation.x_mut() += move_x;
        *translation.y_mut() += move_y;
    }

    if soft_drop_timer.0.finished {
        soft_drop_timer.0.reset();
    }
}

fn rotate_current_tetromino(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Tetromino, &CurrentTetromino, &mut Translation)>
) {
    let mut should_rotate: Option<bool> = None;
    if keyboard_input.just_pressed(KeyCode::X) {
        should_rotate = Some(true);
    }

    if keyboard_input.just_pressed(KeyCode::Z) {
        should_rotate = Some(false);
    }

    if let Some(clockwise) = should_rotate {
        for (mut tetromino, _current, mut translation) in &mut query.iter() {
            let prev_index_x = tetromino.index.x;
            let prev_index_y = tetromino.index.y;

            let matrix_size = Tetromino::SIZES[tetromino.tetromino_type as usize];
            rotate_tetromino_block(&mut tetromino, matrix_size, clockwise);

            *translation.x_mut() +=
                (tetromino.index.x - prev_index_x) as f32 * Block::SIZE;
            *translation.y_mut() +=
                (tetromino.index.y - prev_index_y) as f32 * Block::SIZE;
        }
    }
}

fn keep_current_tetromino_in_bounds(
    mut matrix_query: Query<(&Matrix, &Sprite)>,
    mut current_query: Query<(&CurrentTetromino, &mut Translation)>,
) {
    let mut out_of_bounds: Option<Vec2> = None;

    for (_current, translation) in &mut current_query.iter() {
        for (_matrix, matrix_sprite) in &mut matrix_query.iter() {
            if translation.x() < matrix_sprite.size.x() * -0.5 {
                out_of_bounds = Some(Vec2::new(Block::SIZE * -1.0, 0.0));
                break;
            } else if translation.x() >= matrix_sprite.size.x() * 0.5 {
                out_of_bounds = Some(Vec2::new(Block::SIZE, 0.0));
                break;
            }
        }
    }

    if let Some(difference) = out_of_bounds {
        for (_current, mut translation) in &mut current_query.iter() {
            *translation.x_mut() -= difference.x();
            *translation.y_mut() -= difference.y();
        }
    }
}

fn add_current_to_heap_and_spawn_new(
    mut commands: Commands,
    materials: ResMut<Assets<ColorMaterial>>,
    mut matrix_query: Query<(&Matrix, &Sprite, &Translation)>,
    mut current_query: Query<(Entity, &mut CurrentTetromino, &mut Translation)>,
    mut heap_query: Query<(&Heap, &Translation)>
) {
    let mut should_go_to_heap = false;

    for (_entity, _current_tetromino, current_translation) in &mut current_query.iter() {
        for (_matrix, sprite, _translation) in &mut matrix_query.iter() {
            if current_translation.0.y() <= sprite.size.y() * -0.5 {
                should_go_to_heap = true;
                break;
            }
        }

        for (_heap, heap_translation) in &mut heap_query.iter() {
            if current_translation.0 == heap_translation.0 {
                should_go_to_heap = true;
                break;
            }
        }
    }

    if should_go_to_heap {
        for (entity, _current, mut translation) in &mut current_query.iter() {
            commands.remove_one::<CurrentTetromino>(entity);
            commands.insert_one(entity, Heap);

            *translation.y_mut() += Block::SIZE;
        }

        spawn_current_tetromino(commands, materials);
    }
}

// -----------------
// UTILITY FUNCTIONS
// -----------------

fn rotate_tetromino_block(tetromino_block: &mut Tetromino, matrix_size: i32, clockwise: bool) {
    let orig_x = tetromino_block.index.x;
    let orig_y = tetromino_block.index.y;
    let matrix_size = matrix_size - 1;

    let x = orig_x;
    if clockwise {
        tetromino_block.index.x = orig_y;
        tetromino_block.index.y = matrix_size - x;
    } else {
        tetromino_block.index.x = matrix_size - orig_y;
        tetromino_block.index.y = orig_x;
    }
}

fn spawn_current_tetromino(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let blocks = Tetromino::blocks_from_type(rand::random());
    for block in blocks.into_iter() {
        commands
            .spawn(SpriteComponents {
                material: materials.add(Color::rgb(
                    block.0.color.r,
                    block.0.color.g,
                    block.0.color.b
                ).into()),
                sprite: Sprite {
                    size: Vec2::new(Block::SIZE, Block::SIZE),
                },
                translation: Translation(Vec3::new(
                    (block.1.index.x as f32 - 1.5) * Block::SIZE,
                    (block.1.index.y as f32 + 9.5) * Block::SIZE,
                    1.0,
                )),
                ..Default::default()
            })
            .with_bundle(block)
            .with(CurrentTetromino)
        ;
    }
}

impl Distribution<TetrominoType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TetrominoType {
        match rng.gen_range(0, 7) {
            0 => TetrominoType::Line,
            1 => TetrominoType::Square,
            2 => TetrominoType::T,
            3 => TetrominoType::S,
            4 => TetrominoType::Z,
            5 => TetrominoType::L,
            _ => TetrominoType::J
        }
    }
}
