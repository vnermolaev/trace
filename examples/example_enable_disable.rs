use trace::trace;

fn main() {
    env_logger::init();

    let foo = Foo;
    Foo::foo(2);
    foo.bar(7);

    let bar = Bar;
    Bar::foo(2);
    bar.bar(7);

    enabled_arg(2, 3);
    disabled_arg(3, 2);
}

struct Foo;

#[trace(enable(bar))]
impl Foo {
    fn foo(b: i32) -> i32 {
        b
    }

    fn bar(&self, a: i32) -> i32 {
        a
    }
}

struct Bar;

#[trace(disable(foo))]
impl Bar {
    fn foo(b: i32) -> i32 {
        b
    }

    fn bar(&self, a: i32) -> i32 {
        a
    }
}

#[trace(enable(a))]
fn enabled_arg(a: i32, b: i32) -> i32 {
    a + b
}

#[trace(disable(b))]
fn disabled_arg(a: i32, b: i32) -> i32 {
    a + b
}
