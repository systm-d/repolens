use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=GENERATE_MAN");
    println!("cargo:rerun-if-env-changed=GENERATE_COMPLETIONS");
    println!("cargo:rerun-if-changed=src/cli/mod.rs");
    println!("cargo:rerun-if-changed=src/cli/commands/mod.rs");

    let is_release = env::var("PROFILE").as_deref() == Ok("release");

    // Both man pages and shell completions need the runtime CLI struct, which is
    // not importable from build.rs (the bin crate hasn't been compiled yet). The
    // build script reserves the output directories so packagers can populate them
    // by invoking the hidden runtime commands `repolens generate-man` and
    // `repolens completions <shell>` after the binary is built.
    let want_man = is_release || env::var("GENERATE_MAN").is_ok();
    let want_completions = is_release || env::var("GENERATE_COMPLETIONS").is_ok();

    if want_completions {
        let out_dir = PathBuf::from("target").join("completions");
        let _ = std::fs::create_dir_all(&out_dir);
    }

    let _ = want_man;
}
