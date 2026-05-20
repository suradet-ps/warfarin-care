#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
  if let Err(error) = warfarin_care_lib::run() {
    eprintln!("failed to run tauri application: {error}");
  }
}
