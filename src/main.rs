

mod cpu;
use cpu::State8080;

fn main() {
    let rom = include_bytes!("invaders.rom");
    
    let mut cpu = State8080::load_from_rom(rom, 0, 0);
    
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).expect("Did not enter a correct string");
    let loops: i32 = line
        .trim()
        .parse()
        .unwrap();

    for _ in 0..loops {
        cpu.emulate();
    }    
   
    println!("{}", cpu);
}
