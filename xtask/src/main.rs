use std::io::stdout;

fn main() {
    // clean up old coverage data
    if let Ok(s) = std::fs::read_dir("target/coverage/regular/debug/deps") {
        s.map(|p| p.unwrap().path())
            .filter(|p| p.extension().is_some())
            .filter(|p| {
                let ext = p.extension().unwrap().to_str().unwrap();
                ext.eq("gcda")
            })
            .for_each(|p| std::fs::remove_file(p).unwrap());
    }

    let _ = std::fs::remove_dir_all("target/coverage/report");
    let _ = std::fs::remove_file("safe-vk/default.profraw");

    // let output = std::process::Command::new("cargo")
    //     .arg("build")
    //     .args(&["--package", "safe-vk"])
    //     .env("RUSTFLAGS", "-Zinstrument-coverage")
    //     .env("CARGO_TARGET_DIR", "target/coverage")
    //     .output()
    //     .unwrap()
    //     .stderr;
    // println!("{}", std::str::from_utf8(&output).unwrap());

    // let output = std::process::Command::new("cargo")
    //     .arg("build")
    //     .args(&["--package", "safe-vk"])
    //     .env("CARGO_INCREMENTAL", "0")
    //     .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
    //     .env("RUSTDOCFLAGS", "-Cpanic=abort")
    //     .env("CARGO_TARGET_DIR", "target/coverage/regular")
    //     .output()
    //     .unwrap()
    //     .stderr;
    // println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .args(&["--", "--test-threads=1"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
        .env("RUSTDOCFLAGS", "-Cpanic=abort")
        .env("CARGO_TARGET_DIR", "target/coverage/regular")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .args(&["--", "--test-threads=1"])
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .env("CARGO_TARGET_DIR", "target/coverage/source")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("grcov")
        .arg(".")
        .args(&["-s", "./safe-vk/src"])
        .args(&["--binary-path", "./target/coverage/source/debug/"])
        .args(&["-t", "html"])
        .arg("--branch")
        .arg("--ignore-not-existing")
        .args(&["-o", "target/coverage/report/"])
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());
}
