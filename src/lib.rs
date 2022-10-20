mod simulation;
use simulation::SimulationState;

const TICKS_PER_GAME: usize = 100000;

pub fn run() {
    let mut simulation = SimulationState::new((100, 100));
    for _ in 0..TICKS_PER_GAME {
        simulation.process_tick()
    }
}