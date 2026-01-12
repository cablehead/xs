extern crate pwd;

use pwd::Passwd;

fn main() {
    let iter = Passwd::iter();

    println!("all accounts: ");
    for p in iter {
        println!("{:?}", p);
    }

    let iter = Passwd::iter();

    println!("all accounts that don't start with _: ");
    for p in iter.filter(|q| !q.name.starts_with("_")) {
        println!("{:?}", p);
    }
}
