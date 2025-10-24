use tauri::{
    menu::{Menu, MenuEvent, MenuItem},
    tray::TrayIcon,
    App, Emitter, Runtime,
};

pub fn initialize(app: &mut App) -> tauri::Result<()> {
    let handle = app.handle();
    let menu = Menu::new(app)?;
    let settings = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let logs = MenuItem::with_id(app, "logs", "Logs", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    menu.append(&settings)?;
    menu.append(&logs)?;
    menu.append(&quit)?;

    if let Some(tray) = handle.tray_by_id("main") {
        attach_tray_handlers(tray, menu)?;
    }

    app.emit("tray-ready", ())?;
    Ok(())
}

fn attach_tray_handlers<R: Runtime>(tray: TrayIcon<R>, menu: Menu<R>) -> tauri::Result<()> {
    tray.set_menu(Some(menu))?;
    tray.on_menu_event(|app, event: MenuEvent| match event.id().as_ref() {
        "settings" => {
            let _ = app.emit("open-settings", ());
        }
        "logs" => {
            #[cfg(debug_assertions)]
            {
                crate::output::logs::broadcast_logs(app);
            }
            let _ = app.emit("open-logs", ());
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    });
    Ok(())
}
