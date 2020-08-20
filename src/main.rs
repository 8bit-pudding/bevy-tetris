use bevy::{
    prelude::*,
};
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::collections::HashMap;

fn main() {
    App::build()
        .add_default_plugins()
        .add_resource(SoftDropTimer(Timer::from_seconds(0.750)))
        .add_resource(PrintInfoTimer(Timer::from_seconds(1.0)))
        .add_startup_system(setup.system())
        // .add_system(print_info.system())
        .add_system(move_current_tetromino.system())
        .add_system(update_block_sprites.system())
        .run();
}

struct SoftDropTimer(Timer);

struct PrintInfoTimer(Timer);

// Base entity, everything is made out of blocks
struct Block {
    color: Color,
}

struct Matrix {
    width: i32,
    height: i32,
}

// Holds a block's position within a tetromino for rotation
#[derive(Debug)]
struct MatrixPosition {
    x: i32,
    y: i32,
}

// A block can be part of a tetromino. Stores the block's index within that
// tetromino for the purpose of rotation.
#[derive(Debug)]
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

// A block can be part of one of the tetrominos that are next in line.
struct NextTetromino {
    pos_in_line: u8,
}

// A block can be part of the heap.
struct Heap;

impl Block {
    const SIZE: f32 = 25.0;
}

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>
) {
    let matrix = Matrix {
        width: 10,
        height: 22,
    };

    commands
        .spawn(Camera2dComponents::default())
        .spawn(UiCameraComponents::default())
    ;

    spawn_current_tetromino(&mut commands, &matrix, &mut materials);

    commands
        .spawn(SpriteComponents {
            material: materials.add(Color::rgb(0.0, 0.0, 0.0).into()),
            sprite: Sprite {
                size: Vec2::new(matrix.width as f32 * Block::SIZE, matrix.height as f32 * Block::SIZE),
            },
            ..Default::default()
        })
        .with(matrix)
    ;
}

fn print_info(
    time: Res<Time>,
    mut timer: ResMut<PrintInfoTimer>,
    mut matrix_query: Query<(&Matrix, &Sprite, &Translation)>,
    mut current_query: Query<(&MatrixPosition, &Tetromino, &CurrentTetromino)>
) {
    timer.0.tick(time.delta_seconds);

    if timer.0.finished {
        for (position, tetromino, _current) in &mut current_query.iter() {
            println!("Current matrix_pos: {:?}", position);
            println!("Current tetromino: {:?}", tetromino);
        }
        println!("");
        timer.0.reset();
    }
}

fn move_current_tetromino(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut soft_drop_timer: ResMut<SoftDropTimer>,
    keyboard_input: Res<Input<KeyCode>>,
    mut matrix_query: Query<&Matrix>,
    mut current_query: Query<(Entity, &mut MatrixPosition, &mut Tetromino, &CurrentTetromino)>,
    mut heap_query: Query<(&mut MatrixPosition, &Heap)>
) {
    // Store current positions in map by entity ID
    let mut prev_positions: HashMap<u32, (i32, i32)> = HashMap::new();
    for (entity, position, _tetromino, _current) in &mut current_query.iter() {
        prev_positions.insert(entity.id(), (position.x, position.y));
    }

    if keyboard_input.just_pressed(KeyCode::I) || keyboard_input.just_pressed(KeyCode::Up) {
        while check_tetromino_positions(&mut current_query, &mut heap_query) {
            for (_entity, mut position, _tetromino, _current) in &mut current_query.iter() {
                position.y -= 1;
            }
        }

        for (entity, mut position, _tetromino, _current) in &mut current_query.iter() {
            position.y += 1;
            commands.remove_one::<CurrentTetromino>(entity);
            commands.insert_one(entity, Heap);
        }

        for matrix in &mut matrix_query.iter() {
            spawn_current_tetromino(&mut commands, matrix, &mut materials);
        }

        return;
    }

    // Movement
    soft_drop_timer.0.tick(time.delta_seconds);

    let mut move_x = 0;
    let mut move_y = 0;
    if keyboard_input.just_pressed(KeyCode::J) || keyboard_input.just_pressed(KeyCode::Left) {
        move_x -= 1;
    }

    if keyboard_input.just_pressed(KeyCode::L) || keyboard_input.just_pressed(KeyCode::Right) {
        move_x += 1;
    }

    if keyboard_input.just_pressed(KeyCode::K) || keyboard_input.just_pressed(KeyCode::Down) || soft_drop_timer.0.finished {
        move_y -= 1;
        soft_drop_timer.0.reset();
    }

    if soft_drop_timer.0.finished {
        soft_drop_timer.0.reset();
    }

    let mut should_rotate: Option<bool> = None;
    if keyboard_input.just_pressed(KeyCode::X) {
        should_rotate = Some(true);
    }

    if keyboard_input.just_pressed(KeyCode::Z) {
        should_rotate = Some(false);
    }

    let mut x_over = 0;
    let mut y_over = 0;

    for (_entity, mut position, mut tetromino, _current) in &mut current_query.iter() {
        let mut move_x = move_x;
        let mut move_y = move_y;

        // Rotation
        if let Some(clockwise) = should_rotate {
            let prev_index_x = tetromino.index.x;
            let prev_index_y = tetromino.index.y;

            let matrix_size = Tetromino::SIZES[tetromino.tetromino_type as usize];
            rotate_tetromino_block(&mut tetromino, matrix_size, clockwise);

            move_x += tetromino.index.x - prev_index_x;
            move_y += tetromino.index.y - prev_index_y;
        }

        // Bounds
        for matrix in &mut matrix_query.iter() {
            if position.x + move_x < 0 {
                x_over = (position.x + move_x).min(x_over);

            } else if position.x + move_x >= matrix.width {
                x_over = ((position.x + move_x) - matrix.width + 1).max(x_over);
            }
        }

        position.x += move_x;
        position.y += move_y;
    }

    for (_entity, mut position, mut tetromino, _current) in &mut current_query.iter() {
        position.x -= x_over;
        position.y -= y_over;
    }

    // TODO: Probably better off setting the matrix up so you can index into it to look for occupied spots around the current tetromino
    // Check if any blocks in tetromino are overlapping with heap
    if !check_tetromino_positions(&mut current_query, &mut heap_query) {
        let mut should_revert = true;

        if let Some(_) = should_rotate {
            let try_moves = [
                ( 1,  0),
                ( 2,  0),
                (-1,  0),
                (-2,  0),
                (-1, -2), // T spins
                ( 1, -2),
            ];

            for try_move in try_moves.iter() {
                for (_entity, mut position, _tetromino, _current) in &mut current_query.iter() {
                    position.x += try_move.0;
                    position.y += try_move.1;
                }

                if check_tetromino_positions(&mut current_query, &mut heap_query) {
                    should_revert = false;
                    break;
                }
            }
        } else {
            // Revert movement and add to heap
            for (entity, _position, _tetromino, _current) in &mut current_query.iter() {
                commands.remove_one::<CurrentTetromino>(entity);
                commands.insert_one(entity, Heap);
            }

            for matrix in &mut matrix_query.iter() {
                spawn_current_tetromino(&mut commands, matrix, &mut materials);
            }
        }

        if should_revert {
            for (entity, mut position, _tetromino, _current) in &mut current_query.iter() {
                let prev_position = prev_positions.get(&entity.id()).unwrap();
                position.x = prev_position.0;
                position.y = prev_position.1;
            }
        }
    }
}

fn update_block_sprites(
    mut matrix_query: Query<(&Matrix, &Sprite)>,
    mut block_query: Query<(&MatrixPosition, &mut Translation)>
) {
    for (_matrix, matrix_sprite) in &mut matrix_query.iter() {
        for (position, mut translation) in &mut block_query.iter() {
            let new_x: f32 = ((position.x as f32 * Block::SIZE) - (matrix_sprite.size.x() * 0.5)) + (Block::SIZE * 0.5);
            let new_y: f32 = (matrix_sprite.size.y() * -0.5) + (position.y as f32 * Block::SIZE) + (Block::SIZE * 0.5);

            *translation.x_mut() = new_x;
            *translation.y_mut() = new_y;
        }
    }
}

// ----------------
// UTILITY AND IMPL
// ----------------

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

fn check_tetromino_positions(
    current_query: &mut Query<(Entity, &mut MatrixPosition, &mut Tetromino, &CurrentTetromino)>,
    heap_query: &mut Query<(&mut MatrixPosition, &Heap)>
) -> bool {
    for (_entity, position, _tetromino, _current) in &mut current_query.iter() {
        if position.y < 0 {
            return false;
        }

        for (heap_position, _heap) in &mut heap_query.iter() {
            if position.x == heap_position.x && position.y == heap_position.y {
                return false;
            }
        }
    }

    return true;
}

fn spawn_current_tetromino(
    commands: &mut Commands,
    matrix: &Matrix,
    materials: &mut ResMut<Assets<ColorMaterial>>,
) {
    let blocks = Tetromino::blocks_from_type(rand::random());
    for block in blocks.into_iter() {
        let tetromino_matrix_size = Tetromino::SIZES[block.1.tetromino_type as usize];
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
                translation: Translation(Vec3::new(0.0, 0.0, 1.0)),
                ..Default::default()
            })
            .with(CurrentTetromino)
            .with(MatrixPosition {
                x: block.1.index.x + 3,
                y: matrix.height - tetromino_matrix_size + block.1.index.y,
            })
            .with_bundle(block)
        ;
    }
}

#[derive(Copy, Clone, Debug)]
enum TetrominoType {
    I = 0,
    O = 1,
    T = 2,
    S = 3,
    Z = 4,
    L = 5,
    J = 6,
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
                        index: MatrixPosition {
                            x: index.0,
                            y: index.1,
                        },
                        tetromino_type
                    }
                )
            })
            .collect()
    }
}

impl Distribution<TetrominoType> for Standard {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TetrominoType {
        match rng.gen_range(0, 7) {
            0 => TetrominoType::I,
            1 => TetrominoType::O,
            2 => TetrominoType::T,
            3 => TetrominoType::S,
            4 => TetrominoType::Z,
            5 => TetrominoType::L,
            _ => TetrominoType::J
        }
    }
}
