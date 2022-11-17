use clap::Parser;
use fourmisse_arena::{get_average_score, run};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "WORLD_FILE")]
    world: String,
    #[arg(short, long, value_name = "RED_BRAIN_FILE")]
    black_brain: String,
    #[arg(short, long, value_name = "BLACK_BRAIN_FILE")]
    red_brain: Option<String>,

    #[arg(short, long, value_name = "TICKS_PER_GAME")]
    ticks: Option<usize>,
    #[arg(short, long, value_name = "NUMBER_OF_GAMES")]
    games: Option<usize>,
    #[arg(long)]
    gui: bool,
}

fn main() {
    let args = Args::parse();

    // If only one .brain is specified, we use the same for both teams
    let black_brain_path = args.black_brain;
    let red_brain_path = args.red_brain.unwrap_or_else(|| black_brain_path.clone());

    if args.gui {
        // Launch a game inside the GUI takes priority
        //run_gui(args.world, (red_brain_path, black_brain_path), args.ticks)
    } else if let Some(games) = args.games {
        // We get an average score over a set number of games
        get_average_score(
            args.world,
            (red_brain_path, black_brain_path),
            games,
            args.ticks,
        );
    } else {
        run(args.world, (red_brain_path, black_brain_path), args.ticks)
    }
}
