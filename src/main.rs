use image::{ImageBuffer, Rgba, RgbaImage};
use imageproc::{self, drawing::Canvas, point::Point, rect::Rect};
use macroquad::prelude::{camera::mouse, scene::camera_pos, *};
use std::{cmp::Ordering, collections::VecDeque};

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

fn render(mode: Mode, game_state: &mut GameState, physics_objects: &(Vec<Particle>, Vec<Tether>)) {
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

    // Render the physics objects
    physics_objects.1.iter().for_each(|tether| {
        tether.render(&physics_objects.0);
    });
    physics_objects.0.iter().for_each(|particle| {
        particle.render();
    });

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
                        Color::from_rgba(
                            DRAW_COLOR[0] + 50,
                            DRAW_COLOR[1] + 50,
                            DRAW_COLOR[2] + 50,
                            DRAW_COLOR[3],
                        ),
                    );
                }
                DrawMode::Remove => {
                    draw_circle_lines(
                        mouse_x,
                        mouse_y,
                        game_state.remove_radius,
                        1.,
                        Color::from_rgba(
                            DRAW_COLOR[0] - 50,
                            DRAW_COLOR[1] - 50,
                            DRAW_COLOR[2] - 50,
                            DRAW_COLOR[3],
                        ),
                    );
                }
            };

            // Render the UI
            draw_text(
                "Create Mode",
                6.,
                35.,
                50.,
                Color::from_rgba(237, 229, 76, 235),
            );
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

            // Draw the force tool
            draw_circle_lines(
                mouse_x,
                mouse_y,
                game_state.force_radius,
                1.,
                Color::from_hex(0xE73D71),
            )
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
    force_radius: f32,
}

impl GameState {
    fn new() -> Self {
        GameState {
            draw_mode: DrawMode::Add,
            was_drawing: false,
            last_draw_pos: (0., 0.),
            add_radius: 5.,
            remove_radius: 20.,
            force_radius: 20.,
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

    imageproc::drawing::draw_polygon_mut(
        image,
        &[
            Point::new(corner1.x as i32, corner1.y as i32),
            Point::new(corner2.x as i32, corner2.y as i32),
            Point::new(corner3.x as i32, corner3.y as i32),
            Point::new(corner4.x as i32, corner4.y as i32),
        ],
        color,
    );
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
    while !frontier.is_empty() {
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
    physics_objects: &mut (Vec<Particle>, Vec<Tether>),
) {
    // Handle brush resizing logic
    match game_state.draw_mode {
        DrawMode::Add => {
            game_state.add_radius = (game_state.add_radius + TOOL_SIZING_FACTOR * mouse_wheel().1)
                .clamp(MIN_TOOL_RADIUS, MAX_TOOL_RADIUS)
        }
        DrawMode::Remove => {
            game_state.remove_radius = (game_state.remove_radius
                + TOOL_SIZING_FACTOR * mouse_wheel().1)
                .clamp(MIN_TOOL_RADIUS, MAX_TOOL_RADIUS)
        }
    };

    // Lattice fill
    if is_key_pressed(KeyCode::Enter) {
        physics_objects.0.clear();
        physics_objects.1.clear();
        create_particle_lattice(create_canvas, physics_objects, 10., 10000., 0.);
    }

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
        physics_objects.0.clear();
        physics_objects.1.clear();
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
                DrawMode::Remove => (game_state.remove_radius, Rgba([0, 0, 0, 0])),
            };

            draw_rounded_line(
                create_canvas,
                last_pos,
                new_pos,
                2. * draw_info.0,
                draw_info.1,
            );
        }
        game_state.was_drawing = true;
        // Update last position that was drawn to. (for filling gaps between mouse jumps)
        game_state.last_draw_pos = mouse_position();
    }
    if is_mouse_button_released(MouseButton::Left) {
        game_state.was_drawing = false;
    }
}

fn handle_sim_logic(
    game_state: &mut GameState,
    physics_objects: &mut (Vec<Particle>, Vec<Tether>),
) {
    let (mouse_x, mouse_y) = mouse_position();

    // Force tool resizing
    game_state.force_radius = (game_state.force_radius + TOOL_SIZING_FACTOR * mouse_wheel().1)
        .clamp(MIN_TOOL_RADIUS, MAX_TOOL_RADIUS);

    // Force tool forcing ig
    if is_mouse_button_down(MouseButton::Left) {
        apply_force_from_point(
            physics_objects,
            vec2(mouse_x, mouse_y),
            10000. * game_state.force_radius,
        );
    }
    if is_mouse_button_down(MouseButton::Right) {
        apply_force_from_point(
            physics_objects,
            vec2(mouse_x, mouse_y),
            -10000. * game_state.force_radius,
        );
    }

    update_physics(physics_objects, get_frame_time());
}

fn apply_force_from_point(
    physics_objects: &mut (Vec<Particle>, Vec<Tether>),
    point: Vec2,
    strength: f32,
) {
    physics_objects.0.iter_mut().for_each(|particle| {
        let distance = (particle.position - point).length();
        let direction = (particle.position - point).normalize();
        particle.apply_force(strength * direction / distance.powi(2));
    });
}

fn create_particle_lattice(
    create_canvas: &ImageBuffer<Rgba<u8>, Vec<u8>>,
    physics_objects: &mut (Vec<Particle>, Vec<Tether>),
    hex_radius: f32,
    stiffness: f32,
    damping_constant: f32,
) {
    // Get a vector of valid centerpoints for hexagons in the lattice.
    let dx = hex_radius * 3.;
    let dy = hex_radius * 3.0_f32.sqrt() / 2.;
    let count_x = ((create_canvas.width() as f32 - 1.) / dx) as u32;
    let count_y = ((create_canvas.height() as f32 - 1.) / dy) as u32;

    // Create grid of slots that may or may not be hexagons
    let mut hex_points: Vec<Option<(f32, f32)>> = vec![None; (count_x * count_y) as usize];

    // Fill slots with hexagons with their location in tuple form
    for row_i in 0..count_y {
        let left_pad = (3. / 2.) * hex_radius * (row_i % 2) as f32;
        for column_i in 0..count_x {
            let x = left_pad + dx * column_i as f32;
            let y = dy * row_i as f32;

            if create_canvas.get_pixel(x as u32, y as u32).0 == DRAW_COLOR {
                hex_points[(row_i * count_x + column_i) as usize] = Some((x, y));
            }
        }
    }

    // Create particles for each hexagon vertex, avoiding duplicate particles

    // indices to topleft-topright-midright-bottomright-bottomleft-midleft particles for every placed hex
    let mut hex_particles_indices: Vec<Option<[usize; 6]>> =
        vec![None; (count_x * count_y) as usize];

    let cos60 = 1. / 2.;
    let sin60 = 3.0_f32.sqrt() / 2.;

    hex_points.iter().enumerate().for_each(|(i, hex_point)| {
        if hex_point.is_none() {
            return;
        } // Disregard if no hex in this spot
        let (x, y) = hex_point.unwrap();

        // Index of the hex to the top-left of this hex
        let left_hex_index = match ((i as f32) / (count_x as f32)).floor() as i32 % 2 == 0 {
            true => {
                if i as u32 >= count_x + 1 {
                    i - (count_x + 1) as usize
                } else {
                    usize::MAX
                }
            }
            false => {
                if i as u32 >= count_x {
                    i - count_x as usize
                } else {
                    usize::MAX
                }
            }
        };
        // Index of the hex to the top-right of this hex
        let right_hex_index = match ((i as f32) / (count_x as f32)).floor() as i32 % 2 == 0 {
            true => {
                if i as u32 >= count_x {
                    i - count_x as usize
                } else {
                    usize::MAX
                }
            }
            false => {
                if i as u32 >= count_x - 1 {
                    i - (count_x - 1) as usize
                } else {
                    usize::MAX
                }
            }
        };
        // Index of the hex to the top of this hex
        let top_hex_index = if i as u32 >= 2 * count_x {
            i - 2 * count_x as usize
        } else {
            usize::MAX
        };

        // Is a hex to the left-top
        let is_left = if left_hex_index != usize::MAX {
            hex_points[left_hex_index].is_some()
        } else {
            false
        };
        // Is a hex to the right-top
        let is_right = if right_hex_index != usize::MAX {
            hex_points[right_hex_index].is_some()
        } else {
            false
        };
        // Is a hex above
        let is_top = if top_hex_index != usize::MAX {
            hex_points[top_hex_index].is_some()
        } else {
            false
        };

        // // indexes to topleft-topright-midright-bottomright-bottomleft-midleft particles for this hex
        // let mut hex_particle_indices: [Option<usize>; 6] = [None; 6];

        // indices to topleft-topright-midright-bottomright-bottomleft-midleft particles for this hex
        let mut particle_indices: [usize; 6] = [0; 6];

        // Place these if they haven't been placed in prior iteration
        if !is_left && !is_top {
            let top_left = vec2(x - hex_radius * cos60, y - hex_radius * sin60);
            physics_objects
                .0
                .push(Particle::new(top_left, Vec2::ZERO, 1.));
            particle_indices[0] = physics_objects.0.len() - 1;
        }
        if !is_right && !is_top {
            let top_right = vec2(x + hex_radius * cos60, y - hex_radius * sin60);
            physics_objects
                .0
                .push(Particle::new(top_right, Vec2::ZERO, 1.));
            particle_indices[1] = physics_objects.0.len() - 1;
        }
        if !is_left {
            let mid_left = vec2(x - hex_radius, y);
            physics_objects
                .0
                .push(Particle::new(mid_left, Vec2::ZERO, 1.));
            particle_indices[5] = physics_objects.0.len() - 1;
        }
        if !is_right {
            let mid_right = vec2(x + hex_radius, y);
            physics_objects
                .0
                .push(Particle::new(mid_right, Vec2::ZERO, 1.));
            particle_indices[2] = physics_objects.0.len() - 1;
        }

        // Get the indices of particles from hexagons that placed them first
        if is_left {
            particle_indices[5] = hex_particles_indices[left_hex_index].unwrap()[3];
            particle_indices[0] = hex_particles_indices[left_hex_index].unwrap()[2];
        }
        if is_top {
            if !is_left {
                // Avoid placing particle previously placed
                particle_indices[0] = hex_particles_indices[top_hex_index].unwrap()[4];
            }
            particle_indices[1] = hex_particles_indices[top_hex_index].unwrap()[3];
        }
        if is_right {
            if !is_top {
                // Avoid placing particle previously placed
                particle_indices[1] = hex_particles_indices[right_hex_index].unwrap()[5];
            }
            particle_indices[2] = hex_particles_indices[right_hex_index].unwrap()[4];
        }

        // Unconditionally place because they come in the next iteration (haven't been placed yet no matter what)
        let bottom_left = vec2(x - hex_radius * cos60, y + hex_radius * sin60);
        physics_objects
            .0
            .push(Particle::new(bottom_left, Vec2::ZERO, 1.));
        particle_indices[4] = physics_objects.0.len() - 1;

        let bottom_right = vec2(x + hex_radius * cos60, y + hex_radius * sin60);
        physics_objects
            .0
            .push(Particle::new(bottom_right, Vec2::ZERO, 1.));
        particle_indices[3] = physics_objects.0.len() - 1;

        // Update the hex_particles_index with all the particle indices for this hex.
        hex_particles_indices[i] = Some(particle_indices);
    });

    // Create the tethers for each hexagon, avoiding placing overlapping tethers
    let mut created_tethers: VecDeque<(usize, usize)> = VecDeque::new(); // Indices of particles for created tethers;
    hex_particles_indices
        .iter()
        .enumerate()
        .for_each(|(i, particle_indices_opt)| {
            // Disregard if no hex here
            let particle_indices = match particle_indices_opt {
                Some(indices) => indices,
                None => return,
            };

            // Create tethers if not already created
            for hex_p_idx in 0..5 {
                if !created_tethers
                    .contains(&(particle_indices[hex_p_idx], particle_indices[hex_p_idx + 1]))
                {
                    created_tethers
                        .push_back((particle_indices[hex_p_idx], particle_indices[hex_p_idx + 1]));
                    physics_objects.1.push(Tether::new(
                        particle_indices[hex_p_idx],
                        particle_indices[hex_p_idx + 1],
                        stiffness,
                        damping_constant,
                        &physics_objects.0,
                    ));
                }
            }

            // Make sure their aren't more than two rows of stored tether indices for optimization purposes
            // Only need the two rows above current row to compare placed tethers
            if created_tethers.len() > 2 * count_x as usize {
                created_tethers.pop_front();
            }
        })
}

struct Particle {
    position: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
    mass: f32,
    net_force: Vec2,
    color: Color,
}

impl Particle {
    fn new(position: Vec2, velocity: Vec2, mass: f32) -> Self {
        Self {
            position,
            velocity,
            acceleration: Vec2::ZERO,
            mass,
            net_force: Vec2::ZERO,
            color: Color::from_hex(0xf2df50),
        }
    }

    fn apply_force(&mut self, force: Vec2) {
        self.net_force += force;
    }

    fn update(&mut self, dt: f32) {
        self.acceleration = self.net_force / self.mass;
        self.velocity += self.acceleration * dt;
        self.position += self.velocity * dt;

        // Zero out the net force ever frame
        self.net_force = Vec2::ZERO;
    }

    fn render(&self) {
        draw_circle(self.position.x, self.position.y, 1.5, self.color);
    }
}

struct Tether {
    p1_index: usize,
    p2_index: usize,
    k: f32,
    damping_constant: f32,
    initial_dist: f32,
}

impl Tether {
    fn new(
        p1_index: usize,
        p2_index: usize,
        k: f32,
        damping_constant: f32,
        particle_arr: &[Particle],
    ) -> Self {
        let pos1 = particle_arr[p1_index].position;
        let pos2 = particle_arr[p2_index].position;
        Self {
            p1_index,
            p2_index,
            k,
            damping_constant,
            initial_dist: (pos2 - pos1).length(),
        }
    }

    fn update(&mut self, dt: f32, particle_arr: &mut [Particle]) {
        let (p1, p2) = match self.p1_index.cmp(&self.p2_index) {
            Ordering::Less => {
                let (start, end) = particle_arr.split_at_mut(self.p2_index);
                (&mut start[self.p1_index], &mut end[0])
            }
            Ordering::Greater => {
                let (start, end) = particle_arr.split_at_mut(self.p1_index);
                (&mut end[0], &mut start[self.p2_index])
            }
            Ordering::Equal => panic!("Both particles are the same in a tether."),
        };

        let dist = (p2.position - p1.position).length();
        let tether_direction = (p2.position - p1.position).normalize();

        let dx = dist - self.initial_dist;
        let a = self.initial_dist;
        let f = -self.k * dx - 10. * (a * dx + a - dx) / (dx + a).powi(2) + 10. / a;
        //let f = -self.k * dx;

        p1.apply_force((f + p1.velocity * self.damping_constant) * -tether_direction);
        p2.apply_force((f + p2.velocity * self.damping_constant) * tether_direction);
    }

    fn render(&self, particle_arr: &[Particle]) {
        let p1 = &particle_arr[self.p1_index];
        let p2 = &particle_arr[self.p2_index];
        draw_line(
            p1.position.x,
            p1.position.y,
            p2.position.x,
            p2.position.y,
            0.5,
            Color::from_hex(0xededed),
        );
    }
}

fn update_physics(physics_objects: &mut (Vec<Particle>, Vec<Tether>), dt: f32) {
    physics_objects
        .0
        .iter_mut()
        .for_each(|particle| particle.update(dt));
    physics_objects.1.iter_mut().for_each(|tether| {
        tether.update(dt, &mut physics_objects.0);
    });
}

#[macroquad::main(config_window)]
async fn main() {
    let mut current_mode = Mode::Create;
    let mut game_state = GameState::new();
    let mut create_canvas = RgbaImage::new(screen_width() as u32, screen_height() as u32); // Image for drawing squishies

    // Store all physics objects
    let mut physics_objects: (Vec<Particle>, Vec<Tether>) = (vec![], vec![]);

    let t = Texture2D::from_rgba8(
        create_canvas.width() as u16,
        create_canvas.height() as u16,
        &create_canvas,
    );

    loop {
        clear_background(Color::from_hex(0x0E131F));

        if is_key_pressed(KeyCode::Space) {
            current_mode = switch_modes(current_mode);
        }

        // Handle all logic pertaining to each mode
        match current_mode {
            Mode::Create => {
                handle_create_logic(&mut game_state, &mut create_canvas, &mut physics_objects);
                // Update and draw the draw stuff if on create mode.
                t.update(&Image {
                    bytes: create_canvas.to_vec(),
                    width: create_canvas.width() as u16,
                    height: create_canvas.height() as u16,
                });
                draw_texture(t, 0., 0., Color::from_rgba(255, 255, 255, 255));
            }
            Mode::Sim => handle_sim_logic(&mut game_state, &mut physics_objects),
        };

        // Render the UI on top of the drawing.
        render(current_mode, &mut game_state, &physics_objects);

        next_frame().await
    }
}
