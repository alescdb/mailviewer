use std::{
  env, fs,
  path::{Path, PathBuf},
  process::Command,
};

const APP_ID: &str = "com.monapp.id";
const GETTEXT_PACKAGE: &str = "monapp";
const LOCALEDIR: &str = "dist/share/glib-2.0/schemas";
const PKGDATADIR: &str = "/usr/share/monapp";

fn main() {
  let out_dir = env::var("OUT_DIR").unwrap();
  let project = env::var("CARGO_MANIFEST_DIR").unwrap();
  let src = Path::new(&project).join("src");
  let config_in = Path::new(&src).join("config.rs.in");
  let config_rs = Path::new(&src).join("config.rs");

  config(&config_in, &config_rs);

  println!("Le chemin du projet est : {}", project);

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

fn config(cfg_in: &PathBuf, cfg_out: &PathBuf) {
  let config_in = fs::read_to_string(cfg_in).expect("Échec de la lecture de config.rs.in");
  let version = env::var("CARGO_PKG_VERSION").unwrap();

  let config_out = config_in
    .replace("@APP_ID@", &format!("\"{}\"", APP_ID))
    .replace("@VERSION@", &format!("\"{}\"", &version))
    .replace("@GETTEXT_PACKAGE@", &format!("\"{}\"", GETTEXT_PACKAGE))
    .replace("@LOCALEDIR@", &format!("\"{}\"", LOCALEDIR))
    .replace("@PKGDATADIR@", &format!("\"{}\"", PKGDATADIR));

  let dest_path = Path::new(cfg_out);
  fs::write(&dest_path, config_out).expect("Échec de l'écriture de config.rs");

  println!("cargo:rerun-if-changed={}", cfg_out.to_str().unwrap());
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
