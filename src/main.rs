/// Main entry point for the function
use rust3d::run;

/// main function to start program
fn main() {
    pollster::block_on(run());
}
