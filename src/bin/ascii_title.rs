use figlet_rs::FIGfont;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: ascii-title <text>");
        std::process::exit(1);
    }

    let text = &args[1];

    // Load the standard font
    let font = FIGfont::standard().unwrap();

    if let Some(figure) = font.convert(text) {
        println!("{}", figure);
    } else {
        eprintln!("Failed to convert text to ASCII art");
        std::process::exit(1);
    }
}
