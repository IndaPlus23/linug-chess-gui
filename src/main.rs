use chess_lib::{Game, PieceType, GameState, Color as PieceColor};
use bevy::{
    prelude::*,
    window::WindowResolution,
    sprite::MaterialMesh2dBundle
};
use bevy_svg::prelude::*;
use std::iter::zip;

const DEFAULT_SCREEN_RESOLUTION: (f32, f32) = (1000., 1000.);

fn main() {
    App::new()
        .add_plugins((ChessPlugin, DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(DEFAULT_SCREEN_RESOLUTION.0, DEFAULT_SCREEN_RESOLUTION.1).with_scale_factor_override(1.0),
                title: "".to_string(),
                ..default()
            }),
            ..default()
        }), SvgPlugin))
        .run();
}

#[derive(Resource)]
struct ChessGame(Game);  //used to store the active game of chess

#[derive(Resource)]
struct LastRenderedResolution((f32, f32)); //used to store last rendered resolution

#[derive(Resource)]
struct LastMarkerRenderedResolution((f32, f32)); //used to store last rendered resolution for markers

#[derive(Resource)]
struct RerenderBoard(bool);  //used as a flag to decide if to rerender board next frame (if a move has been played)

#[derive(Resource)]
struct RerenderMarkers(bool);  //used as a flag to decide if to rerender markers next frame (if a move has been played or square has been selected)

#[derive(Resource)]
struct SelectedSquare(Option<u32>);  //used to store the user selected square

#[derive(Resource)]
struct LastMove(Option<(u32, u32)>);  //used to store the last move

#[derive(Component)]
struct Piece;  //unit struct to help identifying the pieces

#[derive(Component)]
struct Chessboard;  //unit struct to help identifying the chessboard

#[derive(Component)]
struct MoveMarker;  //unit struct to help identifying the move markers

#[derive(Component)]
struct SquareMarker;  //unit struct to help identifying the square highlighters


pub struct ChessPlugin;

impl Plugin for ChessPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ChessGame(Game::new()))
        .insert_resource(SelectedSquare(None))
        .insert_resource(LastRenderedResolution(DEFAULT_SCREEN_RESOLUTION))
        .insert_resource(LastMarkerRenderedResolution(DEFAULT_SCREEN_RESOLUTION))
        .insert_resource(RerenderBoard(true))
        .insert_resource(RerenderMarkers(true))
        .insert_resource(LastMove(None))
        .add_systems(Startup, setup)
        .add_systems(Update, (render_board, drag_piece, render_legal_moves, game_is_over));
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn drag_piece(
    mut windows: Query<&mut Window>,
    mouse: Res<Input<MouseButton>>, 
    mut rerender: ResMut<RerenderBoard>,
    mut rerender_markers: ResMut<RerenderMarkers>,
    mut game: ResMut<ChessGame>,
    mut selected_square: ResMut<SelectedSquare>,
    mut last_move: ResMut<LastMove>,) {

    let mut window = windows.single_mut();

    if mouse.just_pressed(MouseButton::Left) {
        if cursor_is_over_board(window.cursor_position().unwrap(), window.width(), window.height()) {
            match selected_square.0 {
                Some(square) => {
                    let to = vec2_to_square(window.cursor_position().unwrap(), window.width(), window.height());
    
                    if game.0.get_legal_moves(square).contains(&to) && game.0.get_game_state() == GameState::InProgress {
                        match &mut game.0.board.squares[square as usize] { //handle promotions
                            Some(piece) => {
                                if piece.piece_type == PieceType::Pawn && (to > 55 || to < 8) {
                                    piece.piece_type = choose_promotion_piece(to, vec2_to_quater_square(window.cursor_position().unwrap(), window.width(), window.height()));
                                }},
                            None => {}
                        }
                        game.0.make_move(square, to);
                        last_move.0 = Some((square, to));
                        selected_square.0 = None;
                        rerender.0 = true;
                    }
                    else {
                        window.cursor.icon = CursorIcon::Grab;
                        let square = vec2_to_square(window.cursor_position().unwrap(), window.width(), window.height());
                        match game.0.board.squares[square as usize] {
                            Some(_) => {selected_square.0 = Some(square)},
                            None => {selected_square.0 = None}
                        }
                        
                    }
                },
                None => {
                    window.cursor.icon = CursorIcon::Grab;
                    let square = vec2_to_square(window.cursor_position().unwrap(), window.width(), window.height());
                        match game.0.board.squares[square as usize] {
                            Some(_) => {selected_square.0 = Some(square)},
                            None => {selected_square.0 = None}
                        }
                }
            }
            rerender_markers.0 = true;
        }
        
    }

    if mouse.just_released(MouseButton::Left) {
        if cursor_is_over_board(window.cursor_position().unwrap(), window.width(), window.height()) {
            window.cursor.icon = CursorIcon::Default;
            match selected_square.0 {
                Some(square) => {
                    let to = vec2_to_square(window.cursor_position().unwrap(), window.width(), window.height());
                
                    if game.0.get_legal_moves(square).contains(&to) && game.0.get_game_state() == GameState::InProgress {
                        match &mut game.0.board.squares[square as usize] { //handle promotions
                            Some(piece) => {
                                if piece.piece_type == PieceType::Pawn {
                                    if to > 55 || to < 8 {
                                        piece.piece_type = choose_promotion_piece(to, vec2_to_quater_square(window.cursor_position().unwrap(), window.width(), window.height()));
                                    }
                                }},
                            None => {}
                        }
                        game.0.make_move(square, to);
                        last_move.0 = Some((square, to));
                        selected_square.0 = None;
                        rerender.0 = true;
                        rerender_markers.0 = true;
                    }
                },
                None => {}
            } 
        }    
    }
}

fn render_board(mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut clear_color: ResMut<ClearColor>,
    board_entities: Query<Entity, Or<(With<Piece>, With<Chessboard>)>>,
    windows: Query<&Window>,
    mut last_resolution: ResMut<LastRenderedResolution>,
    mut rerender: ResMut<RerenderBoard>,
    game: Res<ChessGame>) {

    let window = windows.single();
    if (window.width(), window.height()) != last_resolution.0 || rerender.0{  //we only want to rerender the board if the resolution has changed or if the rerender flag is true
    
    rerender.0 = false;
    last_resolution.0 = (window.width(), window.height());

    board_entities.for_each(|entity| {  //clearing board
        commands.entity(entity).despawn(); 
    }); 
     
    clear_color.0 = Color::BLACK;
    let svg: Handle<Svg> = asset_server.load("resources/blue.svg"); 
    commands.spawn((Chessboard, Svg2dBundle {  //spawning the chessboard
        svg,
        origin: Origin::Center,
        transform: Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(window.height()/8.)), 
        ..Default::default()
    }));

    for i in 0..64 {  //matching all pieces in the chessgame resource with the right assests
        let mut path = "";
        match &game.0.board.squares[i] {
            Some(p) => {
                match p.color {
                    PieceColor::White => {
                        path = match &p.piece_type {
                            PieceType::Pawn => "anarcandy/wP.svg",
                            PieceType::King => "anarcandy/wK.svg",
                            PieceType::Knight => "anarcandy/wN.svg",
                            PieceType::Bishop => "anarcandy/wB.svg",
                            PieceType::Rook => "anarcandy/wR.svg",
                            PieceType::Queen => "anarcandy/wQ.svg",
                        }
                    },
                    PieceColor::Black => {
                        path = match &p.piece_type {
                            PieceType::Pawn => "anarcandy/bP.svg",
                            PieceType::King => "anarcandy/bK.svg",
                            PieceType::Knight => "anarcandy/bN.svg",
                            PieceType::Bishop => "anarcandy/bB.svg",
                            PieceType::Rook => "anarcandy/bR.svg",
                            PieceType::Queen => "anarcandy/bQ.svg",
                        }
                    }
                }
            },
            None => {}
        }
        if path != "" {
            let (x, y) = square_to_cordinates(i as u32, window.height());
            let svg: Handle<Svg> = asset_server.load(path); 
            commands.spawn((Piece, Svg2dBundle { //spawning the chesspiece
            svg,
            origin: Origin::Center,
            transform: Transform::from_xyz(x, y, 1.).with_scale(Vec3::splat((window.height()/512.)/8.)), 
            ..Default::default()
            }));
        }
        
    }
    }
}

fn render_legal_moves(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    markers: Query<Entity, Or<(With<SquareMarker>, With<MoveMarker>)>>,
    windows: Query<&Window>,
    mut game: ResMut<ChessGame>,
    selected_square: Res<SelectedSquare>,
    last_move: Res<LastMove>,
    mut rerender: ResMut<RerenderMarkers>,
    mut last_resolution: ResMut<LastMarkerRenderedResolution>){

    let window = windows.single();
    
    if (window.width(), window.height()) != last_resolution.0 || rerender.0 {
        
        rerender.0 = false;
        last_resolution.0 = (window.width(), window.height());

        markers.for_each(|entity| {
            commands.entity(entity).despawn();
        });

        match selected_square.0 {
            Some(square) => {
                let (x, y) = square_to_cordinates(square, window.height());
                commands.spawn((SquareMarker, MaterialMesh2dBundle {  //this mesh makes the selected square appear darker
                    mesh: meshes.add(shape::Quad::new(Vec2::new(window.height()/8.,window.height()/8.)).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgba(0., 0., 0., 0.5))),
                    transform: Transform::from_translation(Vec3::new(x, y, 0.75)),
                    ..default()
                }));

                let legal_moves = game.0.get_legal_moves(square);

                for m in legal_moves {
                    match &game.0.board.squares[square as usize] {
                        Some(piece) => {
                            if piece.piece_type == PieceType::Pawn && (m > 55 || m < 8) {
                                
                                let offset = window.height()/32.;
                                let promotion_quater_offsets = vec![(-offset, offset),(offset, offset),(-offset, -offset),(offset, -offset)];
                                let promotions = vec!["Q","N","R","B"];
                                let color = if piece.color == PieceColor::White {"w"} else {"b"};

                                for (promotion, (x_offset, y_offset)) in zip(promotions, promotion_quater_offsets) {
                                    let path = format!("anarcandy/{}{}.svg", color, promotion);
                                    let (x, y) = square_to_cordinates(m as u32, window.height());
                                    let svg: Handle<Svg> = asset_server.load(path); 
                                    commands.spawn((MoveMarker, Svg2dBundle { //spawning the chesspiece
                                    svg,
                                    origin: Origin::Center,
                                    transform: Transform::from_xyz(x+ x_offset, y + y_offset, 2.).with_scale(Vec3::splat((window.height()/512.)/16.)), 
                                    ..Default::default()
                                    }));
                                    commands.spawn((SquareMarker, MaterialMesh2dBundle {  //this mesh makes the selected square appear darker
                                        mesh: meshes.add(shape::Quad::new(Vec2::new(window.height()/8.,window.height()/8.)).into()).into(),
                                        material: materials.add(ColorMaterial::from(Color::rgba(0., 0., 0., 0.5))),
                                        transform: Transform::from_translation(Vec3::new(x, y, 1.5)),
                                        ..default()
                                    }));
                                }
                                continue;
                            }
                        },
                        None => {}
                    }
                    match game.0.board.squares[m as usize] {
                        Some(_) => { //capture marker (red)
                            let (x, y) = square_to_cordinates(m, window.height());
                            commands.spawn((MoveMarker, MaterialMesh2dBundle {
                                mesh: meshes.add(shape::Circle::new(window.height()/64.).into()).into(),
                                material: materials.add(ColorMaterial::from(Color::rgba(1., 0., 0., 0.5))),
                                transform: Transform::from_translation(Vec3::new(x, y, 2.)),
                                ..default()
                            }));
                        },
                        None => { //non capture marker (black)
                            let (x, y) = square_to_cordinates(m, window.height());
                            commands.spawn((MoveMarker, MaterialMesh2dBundle {
                                mesh: meshes.add(shape::Circle::new(window.height()/64.).into()).into(),
                                material: materials.add(ColorMaterial::from(Color::rgba(0., 0., 0., 0.5))),
                                transform: Transform::from_translation(Vec3::new(x, y, 2.)),
                                ..default()
                            }));
                        }
                    }
                }

            }
            None => {}
        }
        match last_move.0 {
            Some(m) => {
                let (x, y) = square_to_cordinates(m.0, window.height());
                commands.spawn((SquareMarker, MaterialMesh2dBundle {  //this mesh makes the last move square appear more yellow
                    mesh: meshes.add(shape::Quad::new(Vec2::new(window.height()/8.,window.height()/8.)).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgba(1., 1., 0., 0.2))),
                    transform: Transform::from_translation(Vec3::new(x, y, 0.5)),
                    ..default()
                }));
                let (x, y) = square_to_cordinates(m.1, window.height());
                commands.spawn((SquareMarker, MaterialMesh2dBundle {  //this mesh makes the last move square appear more yellow
                    mesh: meshes.add(shape::Quad::new(Vec2::new(window.height()/8.,window.height()/8.)).into()).into(),
                    material: materials.add(ColorMaterial::from(Color::rgba(1., 1., 0., 0.2))),
                    transform: Transform::from_translation(Vec3::new(x, y, 0.5)),
                    ..default()
                }));
            },
            None => {}
        }
    }
}

fn game_is_over(
    mut game: ResMut<ChessGame>,
    mut last_move: ResMut<LastMove>,
    mut rerender: ResMut<RerenderBoard>,
    mut rerender_markers: ResMut<RerenderMarkers>,
){
    if game.0.get_game_state() == GameState::GameOver { //as of now the game is restarted immediately after checkmate, have not decided yet what should happen instead
        game.0 = Game::new();
        last_move.0 = None;
        (rerender.0, rerender_markers.0) = (true, true);
    }
}

fn vec2_to_square(cursor_pos: Vec2, w_width: f32, w_height: f32) -> u32 {  //returns the chessquare from x and y cordinates
    let (x, y) = (cursor_pos.x, cursor_pos.y);
    let x_relative_to_board_edge = x - ((w_width-w_height)/2.);
    let file = (x_relative_to_board_edge/w_height*8.) as u32;
    let rank = 8-(y/w_height*8.) as u32-1;
    file + 8*rank
}

fn vec2_to_quater_square(cursor_pos: Vec2, w_width: f32, w_height: f32) -> u32 {  //returns the quater chessquare from x and y cordinates
    let (x, y) = (cursor_pos.x, cursor_pos.y);                          //quater square is the selected square on a 16x16 board
    let x_relative_to_board_edge = x - ((w_width-w_height)/2.);
    let file = (x_relative_to_board_edge/w_height*16.) as u32;
    let rank = 16-(y/w_height*16.) as u32-1;
    file + 16*rank
}

fn cursor_is_over_board(cursor_pos: Vec2, w_width: f32, w_height: f32) -> bool { //returns true if the cursor is over the chessboard
    let (x, y) = (cursor_pos.x, cursor_pos.y);
    let x_relative_to_board_edge = x - ((w_width-w_height)/2.);
    x_relative_to_board_edge >= 0. && x_relative_to_board_edge <= w_height && y >= 0. && y <= w_height
}

fn choose_promotion_piece(square: u32, quater_square: u32) -> PieceType { //returns the piecetype given a square and quater_square
    let file = square % 8;
    let rank = (square-file)/8;
    let knight_quater = (file*2+1) + 16*(rank*2+1);
    let bishop_quater = knight_quater - 16;
    let queen_quater = knight_quater-1;
    let rook_quater = queen_quater - 16;

    if quater_square == queen_quater {
        return PieceType::Queen;
    }
    else if quater_square == rook_quater {
        return PieceType::Rook;
    }
    else if quater_square == knight_quater {
        return PieceType::Knight;
    }
    else if quater_square == bishop_quater {
        return PieceType::Bishop;
    }
    PieceType::Pawn  //code should never reach this so lets make it obvious if something goes wrong by promoting to a pawn
}

fn square_to_cordinates(square: u32, window_height: f32) -> (f32, f32) {  //returns the x and y requierd to transform a centered object to a chesssquare
    let x = (window_height/8.) * ((square % 8)+1) as f32 - window_height/2. - window_height/16.;
    let y = (window_height/8.) * ((square / 8)) as f32 - window_height/2. + window_height/16.;
    (x, y)
}