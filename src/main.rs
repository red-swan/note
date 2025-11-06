use std::fs::File;
use std::io::Write;
use std::env;
use opener::open;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
      return Err("Did not specify a note!".into());
    }

    let output_path = env::home_dir().unwrap().join("notes.temp");

    if args.len() == 2 && args[1] == "open" {
      open(&output_path)?;
      return Ok(());
    }

    let arg_line = args[1..].join(" ");
    let mut f = File::options().append(true).create(true).open(output_path)?;
    writeln!(&mut f, "{}", arg_line)?;
    Ok(())
}