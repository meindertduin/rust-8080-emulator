use minifb::{Key, Window, WindowOptions};

mod cpu;
mod space_invader;


fn main() {
    let mut invaders_game_state = space_invader::GameState::new_game();
    let mut window = Window::new(
        "invaders test",
        224,
        256,
        WindowOptions::default(),
    ).unwrap();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        invaders_game_state.next_frame(&mut window);
    }

}
