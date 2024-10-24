use std::{env, fs, path::Path, process::Command};

fn main() {
  // let out_dir = env::var_os("OUT_DIR").unwrap();

  println!("cargo::rerun-if-changed=Makefile");
  println!("cargo::rerun-if-changed=src/window.ui");
  println!("cargo::rerun-if-changed=src/mailviewer.gresource.xml");
  println!("cargo::rerun-if-changed=data/io.github.alescdb.mailviewer.gschema.xml");

  // TODO: this causes rebuild every time
  println!("cargo::rerun-if-changed=dist/share/glib-2.0/schemas/gschemas.compiled");
  println!("cargo::rerun-if-changed=dist/share/mailviewer/mailviewer.gresource");

  let output = Command::new("make")
    .arg("gresources")
    .output()
    .expect("Failed to build resources");
  eprintln!("Build resources : {:?}", output);
}

#[allow(dead_code)]
fn glib_compile_resources() {
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("mailviewer.gresource");

  eprintln!("dest_path : {:?}", dest_path);
  let output = Command::new("glib-compile-resources")
    .arg("--sourcedir=src")
    .arg(format!("--target={}", dest_path.to_str().unwrap()))
    .arg("src/mailviewer.gresource.xml")
    .output()
    .expect("Failed to build schema");

  eprintln!("glib_compile_resources : {:?}", output);
}

#[allow(dead_code)]
fn glib_compile_schemas() {
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir)
    .join("share")
    .join("glib-2.0")
    .join("schemas");

  if !dest_path.exists() {
    fs::create_dir_all(&dest_path).unwrap();
  }

  eprintln!("dest_path : {:?}", dest_path);
  let output = Command::new("glib-compile-schemas")
    .arg(format!("--targetdir={}", dest_path.to_str().unwrap()))
    .arg("data")
    .output()
    .expect("Failed to build schema");

  eprintln!("glib_compile_schemas : {:?}", output);
}
