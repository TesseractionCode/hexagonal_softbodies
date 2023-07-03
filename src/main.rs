use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{self, rect::Rect, drawing::Canvas, point::Point};
use macroquad::prelude::*;

const DRAW_COLOR: [u8; 4] = [88, 96, 117, 255];
const MIN_TOOL_RADIUS: f32 = 1.;
const MAX_TOOL_RADIUS: f32 = 175.;
const TOOL_SIZING_FACTOR: f32 = 0.05;

#[derive(Clone, Copy)]
enum Mode {
    Create,
    Sim,
}

fn config_window() -> Conf {
    Conf {
        window_title: "Hexagonal Softbodies".to_owned(),
        window_resizable: false,
        window_width: 800,
        window_height: 600,
        ..Conf::default()
    } 
}

fn render(mode: Mode, game_state: &mut GameState) {
    let w = screen_width();
    let h = screen_height();
    let mouse_x = mouse_position().0;
    let mouse_y = mouse_position().1;

    // Draw the info bar
    let bar_height = 27_f32;
    draw_rectangle(
        0.,
        h - bar_height,
        w,
        bar_height,
        Color::from_rgba(0, 0, 0, 50),
    );

    // Draw mode specific details
    match mode {
        Mode::Create => {
            // Render create-mode relevant things

            // Render the brush size indicators
            match game_state.draw_mode {
                DrawMode::Add => {
                    draw_circle_lines(
                        mouse_x,
                        mouse_y,
                        game_state.add_radius,
                        1.,
                        Color::from_rgba(DRAW_COLOR[0] + 50, DRAW_COLOR[1] + 50, DRAW_COLOR[2] + 50, DRAW_COLOR[3])
                    );
                },
                DrawMode::Remove => {
                    draw_circle_lines(
                        mouse_x,
                        mouse_y,
                        game_state.remove_radius,
                        1.,
                        Color::from_rgba(DRAW_COLOR[0] - 50, DRAW_COLOR[1] - 50, DRAW_COLOR[2] - 50, DRAW_COLOR[3])
                    );
                },
            };

            // Render the UI
            draw_text("Create Mode", 6., 35., 50., Color::from_rgba(237, 229, 76, 235));
            draw_text(
                "[Space] to Change Modes",
                6.,
                60.,
                23.,
                Color::from_rgba(203, 206, 209, 170),
            );
            draw_text(
                "- (Enter) Compute Lattice",
                9.,
                80.,
                18.,
                Color::from_rgba(203, 206, 209, 140),
            );
            draw_text(
                "- (Backspace) Clear",
                9.,
                100.,
                18.,
                Color::from_rgba(203, 206, 209, 140),
            );
            draw_text(
                "- (Q) Switch Brush (Add/Remove)",
                9.,
                120.,
                18.,
                Color::from_rgba(203, 206, 209, 140),
            );
            draw_text(
                "- (F) Fill",
                9.,
                140.,
                18.,
                Color::from_rgba(203, 206, 209, 140),
            );
            

            draw_text(
                "Scroll to change tool sizes.",
                8.,
                h - 8.,
                23.,
                Color::from_hex(0x777A84),
            );
        }
        Mode::Sim => {
            // Render sim-mode relevant things.
            draw_text("Simulate Mode", 6., 35., 50., Color::from_hex(0xE73D71));
            draw_text(
                "- (Space) Change Modes",
                9.,
                60.,
                23.,
                Color::from_rgba(203, 206, 209, 140),
            );

            draw_text(
                "Scroll to change tool sizes. [Arrow keys to pan. -- Right click to repulse.]",
                8.,
                h - 8.,
                23.,
                Color::from_hex(0x777A84),
            );
        }
    }
}

fn switch_modes(current_mode: Mode) -> Mode {
    match current_mode {
        Mode::Create => Mode::Sim,
        Mode::Sim => Mode::Create,
    }
}

enum DrawMode {
    Add,
    Remove,
}

struct GameState {
    draw_mode: DrawMode,
    was_drawing: bool,
    last_draw_pos: (f32, f32),
    add_radius: f32,
    remove_radius: f32,
}

impl GameState {
    fn new() -> Self {
        GameState {
            draw_mode: DrawMode::Add,
            was_drawing: false,
            last_draw_pos: (0., 0.),
            add_radius: 5.,
            remove_radius: 20.,
        }
    }
}

// I hate lines.
fn draw_rounded_line(
    image: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    pos1: (f32, f32),
    pos2: (f32, f32),
    width: f32,
    color: Rgba<u8>,
) {
    // Draw the end caps
    imageproc::drawing::draw_filled_circle_mut(
        image,
        (pos1.0 as i32, pos1.1 as i32),
        (width / 2.) as i32,
        color,
    );
    imageproc::drawing::draw_filled_circle_mut(
        image,
        (pos2.0 as i32, pos2.1 as i32),
        (width / 2.) as i32,
        color,
    );

    let pos1_vec = vec2(pos1.0, pos1.1);
    let pos2_vec = vec2(pos2.0, pos2.1);
    let distance = pos2_vec.distance(pos1_vec);

    // Avoid trying to draw an empty polygon
    if (distance as i32) < 1 {
        return;
    }

    let line_direction = (pos2_vec - pos1_vec).normalize();
    let line_perp = line_direction.perp();

    let corner1 = pos1_vec + line_perp * width / 2.;
    let corner2 = pos1_vec - line_perp * width / 2.;
    let corner3 = pos2_vec - line_perp * width / 2.;
    let corner4 = pos2_vec + line_perp * width / 2.;

    imageproc::drawing::draw_polygon_mut(image, &[
        Point::new(corner1.x as i32, corner1.y as i32),
        Point::new(corner2.x as i32, corner2.y as i32),
        Point::new(corner3.x as i32, corner3.y as i32),
        Point::new(corner4.x as i32, corner4.y as i32)
    ], color);

    // let num_steps = (pos2_vec - pos1_vec).length() as i32;

    // // Draw between the endcaps
    // for step in 0..num_steps {
    //     let pos = pos1_vec + line_direction * step as f32;
    //     imageproc::drawing::draw_filled_circle_mut(
    //         image,
    //         (pos[0] as i32, pos[1] as i32),
    //         (width / 2.) as i32,
    //         color,
    //     );
    // }
}

// Implementation of the s2sphere flood fill algorithm. https://github.com/qedus/sphere
fn flood_fill(
    create_canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
    start_pos: (u32, u32),
    fill_color: Rgba<u8>,
) {
    let start_color = *create_canvas.get_pixel(start_pos.0, start_pos.1);
    let w = create_canvas.width();
    let h = create_canvas.height();

    // Prevent infinite loops
    if start_color == fill_color {
        return;
    }

    let mut frontier = vec![start_pos];

    // Keep going until algo can't find more unfilled pixels
    while frontier.len() > 0 {

        let (x, y) = frontier.pop().unwrap();
        let this_color = *create_canvas.get_pixel(x, y);

        // Skip branching out from this pixel if it is a "border"
        if this_color != start_color {
            continue;
        }

        // Color the pixel (like infection)
        create_canvas.draw_pixel(x, y, fill_color);

        // Branch out to explore other pixels.
        // Ensures that boundaries are not exceeded.
        if x + 1 < w {
            frontier.push((x + 1, y))
        }
        if x > 0 {
            frontier.push((x - 1, y))
        }
        if y + 1 < h {
            frontier.push((x, y + 1))
        }
        if y > 0 {
            frontier.push((x, y - 1))
        }
    }
}

fn handle_create_logic(
    game_state: &mut GameState,
    create_canvas: &mut ImageBuffer<Rgba<u8>, Vec<u8>>,
) {
    // Handle brush resizing logic
    match game_state.draw_mode {
        DrawMode::Add => game_state.add_radius = (game_state.add_radius + TOOL_SIZING_FACTOR * mouse_wheel().1).clamp(MIN_TOOL_RADIUS, MAX_TOOL_RADIUS),
        DrawMode::Remove => game_state.remove_radius = (game_state.remove_radius + TOOL_SIZING_FACTOR * mouse_wheel().1).clamp(MIN_TOOL_RADIUS, MAX_TOOL_RADIUS),
    };

    // Brush switching
    if is_key_pressed(KeyCode::Q) {
        game_state.draw_mode = match game_state.draw_mode {
            DrawMode::Add => DrawMode::Remove,
            DrawMode::Remove => DrawMode::Add,
        };
    }

    // Do flood fill
    if is_key_pressed(KeyCode::F) {
        flood_fill(
            create_canvas,
            (mouse_position().0 as u32, mouse_position().1 as u32),
            Rgba(DRAW_COLOR),
        );
    }

    // Handle clear request
    if is_key_pressed(KeyCode::Backspace) {
        imageproc::drawing::draw_filled_rect_mut(
            create_canvas,
            Rect::at(0, 0).of_size(create_canvas.width(), create_canvas.height()),
            Rgba([0, 0, 0, 0]),
        );
    }

    // Handle drawing logic
    if is_mouse_button_down(MouseButton::Left) {
        if game_state.was_drawing {
            let last_pos = game_state.last_draw_pos;
            let new_pos = mouse_position();

            let draw_info = match game_state.draw_mode {
                DrawMode::Add => (game_state.add_radius, Rgba(DRAW_COLOR)),
                DrawMode::Remove => (game_state.remove_radius, Rgba([0, 0, 0, 0]))
            };

            draw_rounded_line(create_canvas, last_pos, new_pos, 2. * draw_info.0, draw_info.1);
        }
        game_state.was_drawing = true;
        // Update last position that was drawn to. (for filling gaps between mouse jumps)
        game_state.last_draw_pos = mouse_position();
    }
    if is_mouse_button_released(MouseButton::Left) {
        game_state.was_drawing = false;
    }
}

fn handle_sim_logic() {}

// FG Color: 0xE73D71

#[macroquad::main(config_window)]
async fn main() {
    let mut current_mode = Mode::Create;
    let mut game_state = GameState::new();
    let mut create_canvas = RgbaImage::new(screen_width() as u32, screen_height() as u32); // Image for drawing squishies

    loop {
        clear_background(Color::from_hex(0x0E131F));

        if is_key_pressed(KeyCode::Space) {
            current_mode = switch_modes(current_mode);
        }
        let t = Texture2D::from_rgba8(
            create_canvas.width() as u16,
            create_canvas.height() as u16,
            &create_canvas.to_vec(),
        );
        draw_texture(t, 0., 0., Color::from_rgba(255, 255, 255, 255));

        // Handle all logic pertaining to each mode
        match current_mode {
            Mode::Create => handle_create_logic(&mut game_state, &mut create_canvas),
            Mode::Sim => handle_sim_logic(),
        };

        render(current_mode, &mut game_state);

        next_frame().await
    }
}
