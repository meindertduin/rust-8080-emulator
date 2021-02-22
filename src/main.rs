

mod cpu;
mod space_invader;

use cpu::State8080;

fn main() {
    let rom = include_bytes!("invaders.rom");
    
    let mut cpu = State8080::load_from_rom(rom, 0, 0);
    let mut count: i32 = 0;
    loop {
        count += 1;
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        cpu.emulate();
        println!("{}", cpu);
        println!("count={}", count);
    }
}
