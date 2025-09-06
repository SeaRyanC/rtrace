use clap::Parser;
use rtrace::hello_world;

/// A simple CLI that demonstrates the rtrace library
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name to greet (dummy argument)
    #[arg(short, long, default_value = "world")]
    name: String,

    /// Number of times to repeat (dummy argument)
    #[arg(short, long, default_value_t = 1)]
    count: u8,

    /// Whether to use uppercase (dummy argument)
    #[arg(short, long)]
    uppercase: bool,
}

fn main() {
    let args = Args::parse();

    // Get the hello world message from the library
    let mut message = hello_world();

    // Apply dummy transformations based on CLI args
    if args.uppercase {
        message = message.to_uppercase();
    }

    // Print the message the specified number of times
    for i in 0..args.count {
        if args.count > 1 {
            println!("{}: {}", i + 1, message);
        } else {
            println!("{}", message);
        }
    }

    // Show that we processed the name argument (even though we don't use it)
    if args.name != "world" {
        println!("(Note: Hello to {} as well!)", args.name);
    }
}
