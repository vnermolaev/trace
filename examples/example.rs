#![allow(dead_code)]
#![allow(unused_variables)]
extern crate trace;
use trace::trace;

fn main() {
    env_logger::init();

    // foo(1, 2);
    let a = A::new().unwrap();
    a.test(1, 2);
}

// #[trace(a = "--{}--", disable(b))]
fn foo(a: i32, b: i32) {
    println!("I'm in foo!");
    bar((a, b));
}

// #[trace(disable(a, b), res = "result: {:?}")]
fn bar((a, b): (i32, i32)) -> i32 {
    println!("I'm in bar!");
    if a == 1 {
        2
    } else {
        b
    }
}

#[derive(Debug)]
struct A {}

#[trace(prefix_enter = "A::", prefix_exit = "A::", disable(new))]
impl A {
    fn new() -> Result<Self, u32> {
        Ok(Self {})
    }
    #[trace(prefix_enter = "<method>::", a = "--{}--", disable(b))]
    fn test(&self, a: u32, b: u32) -> u32 {
        10
    }
}
