// error[E0658]: non-builtin inner attributes are unstable (see issue #54726)
// error[E0658]: The attribute `trace` is currently unknown to the compiler and may have meaning added to it in the future (see issue #29642)
//#![trace]

// #![feature(proc_macro_hygiene)]  // to use custom attributes on `mod`

extern crate trace;

use trace::trace;

// error: an inner attribute is not permitted in this context
// error[E0658]: non-builtin inner attributes are unstable (see issue #54726)
//#![trace]

fn main() {
    env_logger::init();

    foo::foo();
    let foo = foo::Foo;
    foo.bar();
}

#[trace(prefix_enter = "foo::", prefix_exit = "foo::")]
mod foo {
    pub(super) fn foo() {
        println!("I'm in foo!");
    }

    pub(super) struct Foo;
    #[trace(prefix_enter = "Foo::", prefix_exit = "Foo::")]
    impl Foo {
        pub(super) fn bar(&self) {}
    }
}
