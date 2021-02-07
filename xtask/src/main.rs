fn main() {
    // clean up old coverage data
    if let Ok(s) = std::fs::read_dir("target/coverage/old-school/target/debug/deps") {
        s.map(|p| p.unwrap().path())
            .filter(|p| p.extension().is_some())
            .filter(|p| {
                let ext = p.extension().unwrap().to_str().unwrap();
                ext.eq("gcda") || ext.eq("gcno")
            })
            .for_each(|p| std::fs::remove_file(p).unwrap());
    }

    let output = std::process::Command::new("cargo")
        .arg("build")
        .args(&["--package", "safe-vk"])
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .env("CARGO_TARGET_DIR", "target/coverage/source-based")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .args(&["--", "--test-threads=1"])
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .env("CARGO_TARGET_DIR", "target/coverage/source-based")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("cargo")
        .arg("build")
        .args(&["--package", "safe-vk"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
        .env("RUSTDOCFLAGS", "-Cpanic=abort")
        .env("CARGO_TARGET_DIR", "target/coverage/old-school")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .args(&["--", "--test-threads=1"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
        .env("RUSTDOCFLAGS", "-Cpanic=abort")
        .env("CARGO_TARGET_DIR", "target/coverage/old-school")
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());

    let output = std::process::Command::new("grcov")
        .arg("safe-vk/default.profraw")
        .args(&["-s", "./safe-vk"])
        .args(&["--binary-path", "target/coverage/old-school/debug/"])
        .args(&["-t", "html"])
        .arg("--branch")
        .arg("--ignore-not-existing")
        .args(&["-o", "target/coverage/"])
        .output()
        .unwrap()
        .stderr;
    println!("{}", std::str::from_utf8(&output).unwrap());
}
