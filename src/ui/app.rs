use crate::models::Disk;
use crate::ui::window::NixDiskManagerWindow;
use crate::utils::{find_missing_partitions, get_disks, parse_nix_filesystems};
use gtk4::glib;
use gtk4::prelude::*;
use libadwaita as adw;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;

pub struct NixDiskManagerApp {
    app: adw::Application,
    #[allow(dead_code)]
    hardware_config_file: PathBuf,
    #[allow(dead_code)]
    hardware_config: Rc<RefCell<String>>,
    #[allow(dead_code)]
    disks: Rc<RefCell<Vec<Disk>>>,
    #[allow(dead_code)]
    must_save: Rc<RefCell<bool>>,
    #[allow(dead_code)]
    windows: Rc<RefCell<Vec<adw::ApplicationWindow>>>,
}

impl NixDiskManagerApp {
    pub fn new() -> Self {
        let app = adw::Application::builder()
            .application_id("org.glfos.nixdiskmanager")
            .build();

        glib::set_application_name("Nix-disk");
        glib::set_prgname(Some("nix-disk"));

        let hardware_config_file = PathBuf::from("/etc/nixos/hardware-configuration.nix");
        let hardware_config = Rc::new(RefCell::new(String::new()));
        let disks = Rc::new(RefCell::new(Vec::new()));
        let must_save = Rc::new(RefCell::new(false));
        let windows: Rc<RefCell<Vec<adw::ApplicationWindow>>> = Rc::new(RefCell::new(Vec::new()));

        // Configure theme to follow system (simple approach)
        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(adw::ColorScheme::Default);

        let app_instance = Self {
            app: app.clone(),
            hardware_config_file: hardware_config_file.clone(),
            hardware_config: hardware_config.clone(),
            disks: disks.clone(),
            must_save: must_save.clone(),
            windows: windows.clone(),
        };

        // Setup activation
        let hardware_config_clone = hardware_config.clone();
        let disks_clone = disks.clone();
        let config_file_clone = hardware_config_file.clone();
        let must_save_clone = must_save.clone();
        let windows_clone = windows.clone();

        app.connect_activate(move |app| {
            Self::on_activate(
                app,
                &config_file_clone,
                &hardware_config_clone,
                &disks_clone,
                &must_save_clone,
                &windows_clone,
            );
        });

        app_instance
    }

    fn on_activate(
        app: &adw::Application,
        config_file: &PathBuf,
        hardware_config: &Rc<RefCell<String>>,
        disks: &Rc<RefCell<Vec<Disk>>>,
        must_save: &Rc<RefCell<bool>>,
        windows: &Rc<RefCell<Vec<adw::ApplicationWindow>>>,
    ) {
        // Load hardware configuration
        if let Ok(config) = fs::read_to_string(config_file) {
            *hardware_config.borrow_mut() = config;
        } else {
            eprintln!("Failed to read hardware configuration file");
            return;
        }

        // Parse disks
        let config_ref = hardware_config.borrow();
        match get_disks(Some(&config_ref)) {
            Ok(parsed_disks) => {
                // Check for missing partitions
                let configured_partitions = match parse_nix_filesystems(&config_ref) {
                    Ok(parts) => parts.into_values().collect::<Vec<_>>(),
                    Err(e) => {
                        eprintln!("Failed to parse filesystems: {}", e);
                        Vec::new()
                    }
                };

                let missing = find_missing_partitions(&configured_partitions, &parsed_disks);

                *disks.borrow_mut() = parsed_disks;

                // Create and show window
                let window = NixDiskManagerWindow::new(
                    app,
                    disks.clone(),
                    hardware_config.clone(),
                    config_file.clone(),
                    must_save.clone(),
                    true, // Skip welcome dialog
                );

                // Store window reference for theme updates
                windows.borrow_mut().push(window.gtk_window().clone());

                window.present();

                // Show missing partitions dialog AFTER window is presented
                // This ensures it appears on top
                if !missing.is_empty() {
                    window.show_missing_partitions_dialog(&missing);
                }
            }
            Err(e) => {
                eprintln!("Failed to get disks: {}", e);
            }
        }
    }

    pub fn run(&self) -> i32 {
        self.app.run().into()
    }
}
