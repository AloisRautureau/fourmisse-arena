mod simulation;
use simulation::Simulation;

const DEFAULT_TICKS: usize = 100000;

// Runs one game given a world, brains files, as well as the number of ticks per game
// (defaulting to DEFAULT_TICKS)
pub fn run(world: String, brains: (String, String), ticks: Option<usize>) {
    let mut simulation = Simulation::new(
        &world,
        &brains.0,
        &brains.1,
    );

    for _ in 0..ticks.unwrap_or(DEFAULT_TICKS) {
        simulation.process_tick()
    }

    let (red_points, black_points) = simulation.points();
    if red_points > black_points {
        println!("Red ants won with {} against {} for black ants", red_points, black_points)
    } else if black_points > red_points {
        println!("Black ants won with {} against {} for red ants", black_points, red_points)
    } else {
        println!("It's a draw! Both teams got {} points", black_points)
    }
}

// Returns the average score between two brains over a given number of games in a given world
pub fn get_average_score(world: String, brains: (String, String), games: usize, ticks: Option<usize>) {
    // If the number of games is uneven, we'll play one more
    let games = if games % 2 != 0 {
        games + 1
    } else {
        games
    };

    let mut total_score_red = (0, 0);
    let mut total_score_black = (0, 0);
    for g in 0..games {
        let mut simulation = Simulation::new(
            &world,
            if g % 2 == 0 { &brains.0 } else { &brains.1 },
            if g % 2 == 0 { &brains.1 } else { &brains.0 },
        );

        for _ in 0..ticks.unwrap_or(DEFAULT_TICKS) {
            simulation.process_tick()
        }

        let (red_points, black_points) = simulation.points();
        if g % 2 == 0 {
            total_score_red.0 += red_points;
            total_score_black.1 += black_points;
        } else {
            total_score_red.1 += red_points;
            total_score_black.0 += black_points;
        }
    }

    let average_red = (total_score_red.0 / (games as u32 / 2), total_score_red.1 / (games as u32 / 2));
    let average_black = (total_score_black.0 / (games as u32 / 2), total_score_black.1 / (games as u32 / 2));
    let average = ((total_score_red.0 + total_score_black.0) / games as u32, (total_score_red.1 + total_score_black.1) / games as u32);
    println!("Brain {} averaged:\n- {} points as red\n- {} points as black\n- {} points total", brains.0, average_red.0, average_black.0, average.0);
    println!("Brain {} averaged:\n- {} points as red\n- {} points as black\n- {} points total", brains.1, average_red.1, average_black.1, average.1);
}