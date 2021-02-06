fn main() {
    std::process::Command::new("cargo")
        .arg("build")
        .args(&["--package", "safe-vk"])
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .output()
        .unwrap();

    std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .env("RUSTFLAGS", "-Zinstrument-coverage")
        .output()
        .unwrap();

    std::process::Command::new("cargo")
        .arg("build")
        .args(&["--package", "safe-vk"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
        .env("RUSTDOCFLAGS", "-Cpanic=abort")
        .output()
        .unwrap();

    std::process::Command::new("cargo")
        .arg("test")
        .args(&["--package", "safe-vk"])
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Zprofile -Ccodegen-units=1 -Copt-level=0 -Clink-dead-code -Coverflow-checks=off -Zpanic_abort_tests -Cpanic=abort")
        .env("RUSTDOCFLAGS", "-Cpanic=abort")
        .output()
        .unwrap();

    std::process::Command::new("grcov")
        .arg(".")
        .args(&["-s", "."])
        .args(&["--binary-path", "./target/debug/"])
        .args(&["-t", "html"])
        .arg("--branch")
        .arg("--ignore-not-existing")
        .args(&["-o", "./target/debug/coverage/"])
        .output()
        .unwrap();
}
