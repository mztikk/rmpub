use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use structopt::StructOpt;

#[derive(Deserialize, Serialize)]
struct Package {
    name: String,
    version: String,
    id: String,
}

#[derive(Deserialize, Serialize)]
struct Metadata {
    #[serde(default = "Vec::new")]
    packages: Vec<Package>,
    target_directory: String,
    #[serde(default = "Vec::new")]
    workspace_members: Vec<String>,
}

impl Metadata {
    fn get_workspace_package(&self) -> Option<&Package> {
        for member in &self.workspace_members {
            for package in &self.packages {
                if package.id.eq(member) {
                    return Some(package);
                }
            }
        }

        None
    }
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(parse(from_os_str))]
    publish_dir: PathBuf,
}

fn shell_command() -> std::process::Command {
    if cfg!(windows) {
        let mut cmd = std::process::Command::new("cmd");
        cmd.arg("/C");
        cmd
    } else {
        let mut cmd = std::process::Command::new("sh");
        cmd.arg("-c");
        cmd
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Cli::from_args();

    // let current_dir = std::env::current_dir()?;
    let output = shell_command()
        .arg("cargo metadata --format-version=1")
        .output()?;
    let meta_json = String::from_utf8_lossy(&output.stdout);
    let meta: Metadata = serde_json::from_str(&meta_json)?;
    let package = meta.get_workspace_package().unwrap();
    println!("{}", package.id);

    let publish_path = Path::new(&args.publish_dir)
        .join(&package.name)
        .join(&package.version);

    println!("{}", publish_path.to_string_lossy());

    // linux binary has no extension, while windows has .exe
    let file_candidates = vec![package.name.to_string(), format!("{}.exe", package.name)];

    // assume
    // | release
    // | x86_64-pc-windows-gnu
    // | x86_64-unknown-linux-gnu
    // | ...

    let target_dir = &meta.target_directory;
    for entry in std::fs::read_dir(target_dir)? {
        let entry = entry?;
        let compile_path = entry.path();

        if !compile_path.is_dir() {
            continue;
        }

        let compile_dir = compile_path.file_stem().unwrap();

        // ignore debug and release dirs and only publish target compiled versions
        if compile_dir.eq("debug") || compile_dir.eq("release") {
            continue;
        }
        println!("{}", compile_dir.to_string_lossy());

        let compile_path = compile_path.join("release");
        if !compile_path.exists() {
            println!(
                "'{}' was not compiled in release mode",
                compile_dir.to_string_lossy()
            );
            continue;
        }

        let publish_dir = publish_path.join(compile_dir);
        for fc in &file_candidates {
            let file = compile_path.join(fc);
            if !file.exists() {
                continue;
            }

            let publish_path = publish_dir.join(fc);
            // dont overwrite same version(even if it may be diff)
            if publish_path.exists() {
                println!(
                    "{} already exists with version {} and target {}",
                    package.name,
                    package.version,
                    compile_dir.to_string_lossy()
                );
                continue;
            }

            if !publish_dir.exists() {
                std::fs::create_dir_all(&publish_dir)?;
            }
            println!("{}", publish_path.to_string_lossy());
            std::fs::copy(file, publish_path)?;

            // there shouldnt be another
            break;
        }
    }

    Ok(())
}
