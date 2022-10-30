use fourmisse_arena::{get_average_score, run};
use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "WORLD_FILE")]
    world: String,
    #[arg(short, long, value_name = "RED_BRAIN_FILE")]
    red_brain: String,
    #[arg(short, long, value_name = "BLACK_BRAIN_FILE")]
    black_brain: String,

    #[arg(short, long, value_name = "TICKS_PER_GAME")]
    ticks: Option<usize>,
    #[arg(short, long, value_name = "NUMBER_OF_GAMES")]
    games: Option<usize>
}

fn main() {
    let args = Args::parse();

    if let Some(games) = args.games {
        get_average_score(args.world, (args.red_brain, args.black_brain), games, args.ticks);
    } else {
        run(args.world, (args.red_brain, args.black_brain), args.ticks)
    }
}
