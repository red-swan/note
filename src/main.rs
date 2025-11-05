use std::fs::File;
use std::io::Write;
use std::env;

fn main() -> std::io::Result<()> {

    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
      panic!("Did not specify a note!")
    }

    let output_path = env::home_dir().unwrap().join("notes.temp");
    println!("{:?}", output_path); 

    // if args.len() == 2 && args[1] == "open" {
    //   std::fs::File::open(output_path)
    // }

    let arg_line = args[1..].join(" ");
    let mut f = File::options().append(true).create(true).open(output_path)?;
    writeln!(&mut f, "{:?}", arg_line)?;
    Ok(())
}