extern crate pwd;
use pwd::Passwd;

fn main() {
    let me = Passwd::current_user().expect("Could not get current user");
    println!(
        "my username is {}, home directory is {}, and my shell is {}. My uid/gid are {}/{}",
        me.name, me.dir, me.shell, me.uid, me.gid
    );
}
