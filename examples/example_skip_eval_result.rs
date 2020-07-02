use std::time::Duration;
use trace::trace;

fn main() {
    env_logger::init();

    A::returns_convoluted_result(1);
}

struct A;

#[trace(prefix = "A::", disable(new))]
impl A {
    #[trace(disable(res))]
    fn returns_convoluted_result(a: u8) -> (Duration, u8, B) {
        (Duration::from_secs(20), a, B)
    }
}

#[derive(Debug)]
struct B;
