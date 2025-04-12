pub use processor::srm_run;
mod cli;
mod error;
mod processor;

fn main() {
    srm_run();
}
