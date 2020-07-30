use std::process::Command;

fn main() {
    let mut child = Command::new("flatc")
        .args(&["-o", "src", "flatbuf/minitrace.fbs", "--rust"])
        .spawn()
        .expect("fail to start flatc");

    child.wait().expect("flatc wasn't running");
}
