#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::error::Error;

use tauri::{async_runtime, generate_context, AppHandle, Builder};
use tauri_plugin_updater::UpdaterExt;

fn run() {
    Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            #[allow(unused_variables)]
            let handle = app.handle().clone();
            async_runtime::spawn(async move {
                #[cfg(not(debug_assertions))]
                update(handle).await.unwrap();
            });

            Ok(())
        })
        .run(generate_context!())
        .expect("error while running tauri application");
}

#[allow(dead_code)]
async fn update(app: AppHandle) -> Result<(), Box<dyn Error>> {
    // TODO: move ts to another module
    if let Some(update) = app
        .updater_builder()
        .pubkey(include_str!("../../../PolyLauncher.key.pub"))
        .build()?
        .check()
        .await?
    {
        let mut downloaded = 0;

        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    println!("downloaded {downloaded} from {content_length:?}");
                },
                || {
                    println!("download finished");
                },
            )
            .await?;

        println!("update installed");
        app.restart();
    }

    Ok(())
}

fn main() {
    run()
}
