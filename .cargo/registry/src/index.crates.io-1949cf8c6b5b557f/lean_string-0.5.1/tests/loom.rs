// RUSTFLAGS="--cfg loom" cargo test --test loom --release --features loom -- --test-threads=1
#![cfg(loom)]

use lean_string::LeanString;
use loom::{thread, thread::JoinHandle};
use paste::paste;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

macro_rules! test_model {
    (
        run: $run:block
        fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) $(-> $ret:ty)? $body:block
    ) => {
        paste! {
            #[test]
            fn [<run_ $name _model>]() {
                loom::model(|| {
                    let _profiler = dhat::Profiler::builder().testing().build();
                    $run
                    let stats = dhat::HeapStats::get();
                    // https://github.com/tokio-rs/loom/issues/369
                    dhat::assert_eq!(stats.curr_blocks, 1);
                })
            }
        }
        fn $name( $($arg : $aty),* ) $(-> $ret)? {
            $body
        }
    }
}

test_model! {
    run: {
        push2().join().unwrap();
    }
    fn push2() -> JoinHandle<()> {
        let mut one = LeanString::from("12345678901234567890");
        let two = one.clone();

        let th = thread::spawn(move || {
            let mut three = two.clone();
            three.push('a');

            assert_eq!(two, "12345678901234567890");
            assert_eq!(three, "12345678901234567890a");
        });

        one.push('a');
        assert_eq!(one, "12345678901234567890a");

        th
    }
}

test_model! {
    run: {
        remove2().join().unwrap();
    }
    fn remove2() -> JoinHandle<()> {
        let mut one = LeanString::from("abcdefghijklmnopqrstuvwxyz");
        let two = one.clone();

        let th = thread::spawn(move || {
            let mut three = two.clone();
            assert_eq!(three.remove(3), 'd');
            assert_eq!(two, "abcdefghijklmnopqrstuvwxyz");
            assert_eq!(three, "abcefghijklmnopqrstuvwxyz");
        });

        assert_eq!(one.remove(3), 'd');
        assert_eq!(one, "abcefghijklmnopqrstuvwxyz");

        th
    }
}
