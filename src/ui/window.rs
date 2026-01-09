use crate::models::{Disk, Partition};
use crate::ui::dialogs::{MissingPartitionsDialog, WelcomeDialog};
use crate::ui::widgets::DisksWidget;
use crate::utils::get_nix_disks_config;
use gettextrs::gettext;
use gtk4::prelude::*;
use gtk4::{gio, glib, Button};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::rc::Rc;

pub struct NixDiskManagerWindow {
    window: adw::ApplicationWindow,
    disks: Rc<RefCell<Vec<Disk>>>,
    hardware_config: Rc<RefCell<String>>,
    config_file: PathBuf,
    must_save: Rc<RefCell<bool>>,
    rebuild_banner: adw::Banner,
    rebuild_error_banner: adw::Banner,
    disks_widget: DisksWidget,
    toast_overlay: adw::ToastOverlay,
}

impl NixDiskManagerWindow {
    pub fn new(
        app: &adw::Application,
        disks: Rc<RefCell<Vec<Disk>>>,
        hardware_config: Rc<RefCell<String>>,
        config_file: PathBuf,
        must_save: Rc<RefCell<bool>>,
        skip_welcome: bool,
    ) -> Rc<Self> {
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(&format!("Nix-disk v{}", env!("CARGO_PKG_VERSION")))
            .default_width(800)
            .default_height(600)
            .icon_name("nix-disk")
            .resizable(true)
            .build();

        // Ensure window can be maximized properly
        window.set_default_size(800, 600);

        // Create main layout
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);

        // Create banners
        let rebuild_banner = adw::Banner::new(&gettext("Rebuilding NixOS configuration..."));
        rebuild_banner.set_revealed(false);

        let rebuild_error_banner = adw::Banner::new(&gettext("Failed to rebuild NixOS configuration"));
        rebuild_error_banner.set_revealed(false);
        rebuild_error_banner.add_css_class("error");

        main_box.append(&rebuild_banner);
        main_box.append(&rebuild_error_banner);

        // Create toolbar
        let header_bar = adw::HeaderBar::new();
        main_box.append(&header_bar);

        // Create toast overlay for notifications
        let toast_overlay = adw::ToastOverlay::new();

        // Create content area with scrolled window
        let scrolled = gtk4::ScrolledWindow::builder()
            .hexpand(true)
            .vexpand(true)
            .propagate_natural_width(true)
            .propagate_natural_height(true)
            .build();

        // Create content box for disks and save button
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 20);
        content_box.set_vexpand(true);
        content_box.set_valign(gtk4::Align::Fill);

        // Add top spacer to center content vertically
        let top_spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        top_spacer.set_vexpand(true);
        content_box.append(&top_spacer);

        // Create disks widget with hardware config
        let disks_widget = DisksWidget::new_with_config(disks.clone(), Some(hardware_config.clone()));

        // Set up save callback for the disks widget
        let disks_for_save = disks.clone();
        let hardware_config_for_save = hardware_config.clone();
        let config_file_for_save = config_file.clone();
        let rebuild_banner_for_save = rebuild_banner.clone();
        let rebuild_error_banner_for_save = rebuild_error_banner.clone();
        let must_save_for_save = must_save.clone();
        let disks_widget_for_refresh = disks_widget.clone();

        disks_widget.set_on_save_callback(move || {
            eprintln!("🔘 Callback de sauvegarde appelée!");

            // Create refresh callback that will be called after rebuild completes
            let disks_widget_clone = disks_widget_for_refresh.clone();
            let refresh_callback = Rc::new(move || {
                eprintln!("🔔 Callback de rafraîchissement appelée depuis do_save_config!");
                disks_widget_clone.refresh();
                eprintln!("✅ Rafraîchissement du widget terminé");
            });

            Self::do_save_config(
                &config_file_for_save,
                &disks_for_save,
                &hardware_config_for_save,
                &rebuild_banner_for_save,
                &rebuild_error_banner_for_save,
                &must_save_for_save,
                Some(refresh_callback),
            );
        });

        content_box.append(&disks_widget.widget());

        // Adjust window size based on number of visible disks and partition count
        let num_disks = disks_widget.count_visible_disks();
        if num_disks > 0 {
            // Each card is 300px wide + 20px spacing
            // Formula: (num_disks * 300) + ((num_disks - 1) * 20) + margins (100px)
            let calculated_width = (num_disks as i32 * 320) + 80;
            let window_width = calculated_width.max(800).min(1920); // Min 800px, max 1920px

            // Calculate height based on max card height
            // Header bar (~50px) + banners + top/bottom spacers + margins + save button (~80px)
            let max_card_height = disks_widget.get_max_card_height();
            let calculated_height = max_card_height + 200; // Add space for UI elements
            let window_height = calculated_height.max(600).min(1200); // Min 600px, max 1200px

            window.set_default_size(window_width, window_height);
            eprintln!("📐 Ajustement de la fenêtre pour {} disque(s): {}px de large × {}px de haut",
                     num_disks, window_width, window_height);
        }

        // Add bottom spacer to center content vertically
        let bottom_spacer = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        bottom_spacer.set_vexpand(true);
        content_box.append(&bottom_spacer);

        scrolled.set_child(Some(&content_box));

        // Wrap scrolled in toast overlay
        toast_overlay.set_child(Some(&scrolled));
        main_box.append(&toast_overlay);

        window.set_content(Some(&main_box));

        let window_rc = Rc::new(Self {
            window: window.clone(),
            disks: disks.clone(),
            hardware_config: hardware_config.clone(),
            config_file,
            must_save,
            rebuild_banner,
            rebuild_error_banner,
            disks_widget,
            toast_overlay: toast_overlay.clone(),
        });

        // Fix minimization bug with pkexec: force redraw when window is shown
        let content_box_clone = content_box.clone();
        let scrolled_clone = scrolled.clone();
        window.connect_is_active_notify(move |_| {
            // Force queue a resize and redraw when window becomes active
            content_box_clone.queue_resize();
            content_box_clone.queue_draw();
            scrolled_clone.queue_resize();
            scrolled_clone.queue_draw();
        });

        // Show welcome dialog only if not skipping
        if !skip_welcome {
            let welcome = WelcomeDialog::new();
            welcome.present(Some(&window));
        }

        window_rc
    }


    pub fn show_missing_partitions_dialog(&self, missing: &[Partition]) {
        let dialog = MissingPartitionsDialog::new(missing, self.disks.clone());
        dialog.present(Some(&self.window));
    }

    fn do_save_config(
        config_file: &PathBuf,
        disks: &Rc<RefCell<Vec<Disk>>>,
        hardware_config: &Rc<RefCell<String>>,
        rebuild_banner: &adw::Banner,
        rebuild_error_banner: &adw::Banner,
        must_save: &Rc<RefCell<bool>>,
        on_rebuild_complete: Option<Rc<dyn Fn()>>,
    ) {
        eprintln!("=== Début de la sauvegarde ===");

        let config = hardware_config.borrow().clone();
        let disks_data = disks.borrow().clone();

        // Debug: afficher les disques et leurs points de montage
        eprintln!("Disques à sauvegarder:");
        for disk in &disks_data {
            eprintln!("  Disque: {}", disk.path.display());
            for part in &disk.partitions {
                eprintln!("    Partition: {}", part.path.display());
                eprintln!("      Points de montage: {:?}", part.mount_points);
            }
        }

        match get_nix_disks_config(&config, &disks_data) {
            Ok(new_config) => {
                eprintln!("Configuration générée avec succès");
                eprintln!("Écriture dans: {}", config_file.display());

                if let Err(e) = fs::write(config_file, &new_config) {
                    eprintln!("❌ Erreur d'écriture du fichier: {}", e);
                    rebuild_error_banner.set_revealed(true);
                    return;
                }

                eprintln!("✓ Fichier écrit avec succès");
                eprintln!("Aperçu de la configuration:");
                eprintln!("{}", &new_config[..new_config.len().min(500)]);

                rebuild_error_banner.set_revealed(false);
                rebuild_banner.set_revealed(true);

                // Run nixos-rebuild in background
                let rebuild_banner = rebuild_banner.clone();
                let rebuild_error_banner = rebuild_error_banner.clone();
                let _must_save = must_save.clone();
                let disks_for_reload = disks.clone();
                let hardware_config_for_reload = hardware_config.clone();
                let config_file_for_reload = config_file.clone();

                glib::spawn_future_local(async move {
                    eprintln!("🔄 Lancement de nixos-rebuild switch...");
                    let result = gio::spawn_blocking(|| {
                        // Create a temporary wrapper script for rebuild
                        let timestamp = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let wrapper_path = format!("/tmp/nix_disk_manager_rebuild_{}.sh", timestamp);
                        let status_file = format!("/tmp/nix_disk_manager_rebuild_{}.done", timestamp);

                        let script_content = format!(
                            r#"#!/usr/bin/env bash

echo "======================================"
echo "  RECONSTRUCTION DE LA CONFIGURATION"
echo "======================================"
echo ""

# Preserve environment for sudo
sudo -E nixos-rebuild switch
EXIT_CODE=$?

if [ $EXIT_CODE -eq 0 ]; then
    echo ""
    echo "======================================"
    echo "  ✅ REBUILD TERMINÉ AVEC SUCCÈS"
    echo "======================================"

    # Signal completion
    touch {}
else
    echo ""
    echo "======================================"
    echo "  ❌ ERREUR LORS DU REBUILD"
    echo "======================================"
fi

echo ""
echo "Appuyez sur Entrée ou fermez cette fenêtre..."
read -t 300 || true
"#,
                            status_file
                        );

                        if let Err(e) = std::fs::write(&wrapper_path, script_content) {
                            eprintln!("❌ Erreur: impossible d'écrire le script rebuild: {}", e);
                            return (false, status_file.clone(), wrapper_path.clone());
                        }

                        if let Err(e) = Command::new("chmod").arg("+x").arg(&wrapper_path).status() {
                            eprintln!("❌ Erreur chmod: {}", e);
                            let _ = std::fs::remove_file(&wrapper_path);
                            return (false, status_file.clone(), wrapper_path.clone());
                        }

                        // Try multiple terminals in order of preference
                        let terminals: Vec<(&str, Vec<&str>)> = vec![
                            ("kgx", vec!["--", &wrapper_path]), // GNOME Console
                            ("gnome-terminal", vec!["--", &wrapper_path]),
                            ("konsole", vec!["-e", &wrapper_path]),
                            ("xfce4-terminal", vec!["-e", &wrapper_path]),
                            ("alacritty", vec!["-e", &wrapper_path]),
                            ("kitty", vec![&wrapper_path]),
                            ("xterm", vec!["-e", &wrapper_path]),
                        ];

                        for (term, args) in terminals {
                            eprintln!("⚠️ Tentative avec {}...", term);
                            if Command::new(term).args(&args).spawn().is_ok() {
                                eprintln!("✅ Terminal {} ouvert avec succès", term);
                                return (true, status_file, wrapper_path);
                            }
                        }

                        eprintln!("❌ Aucun terminal trouvé pour exécuter nixos-rebuild");
                        let _ = std::fs::remove_file(&wrapper_path);
                        (false, status_file, wrapper_path)
                    })
                    .await
                    .unwrap_or((false, String::new(), String::new()));

                    let (terminal_opened, status_file_path, script_path) = result;

                    if !terminal_opened {
                        rebuild_banner.set_revealed(false);
                        rebuild_error_banner.set_revealed(true);
                    } else {
                        // Start watching for completion
                        let rebuild_banner_watch = rebuild_banner.clone();
                        let rebuild_error_banner_watch = rebuild_error_banner.clone();
                        let disks_watch = disks_for_reload.clone();
                        let hardware_config_watch = hardware_config_for_reload.clone();
                        let on_rebuild_complete_watch = on_rebuild_complete.clone();
                        let config_file_watch = config_file_for_reload.clone();
                        let check_count = Rc::new(RefCell::new(0u32));

                        glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                            *check_count.borrow_mut() += 1;
                            let count = *check_count.borrow();

                            // Check if status file exists
                            if std::path::Path::new(&status_file_path).exists() {
                                eprintln!("✅ Rebuild terminé détecté!");

                                // Reload hardware config from disk (it was updated by the rebuild)
                                eprintln!("📄 Rechargement de la config depuis: {}", config_file_watch.display());
                                let updated_config = std::fs::read_to_string(&config_file_watch)
                                    .unwrap_or_else(|e| {
                                        eprintln!("❌ Erreur lecture config: {}", e);
                                        hardware_config_watch.borrow().clone()
                                    });

                                // Update the config in memory
                                *hardware_config_watch.borrow_mut() = updated_config.clone();
                                eprintln!("✅ Config en mémoire mise à jour");

                                // Reload disks from system with the updated config
                                use crate::utils::get_disks;
                                if let Ok(new_disks) = get_disks(Some(&updated_config)) {
                                    *disks_watch.borrow_mut() = new_disks;
                                    eprintln!("✅ Liste des disques rechargée après rebuild");
                                }

                                // Call the refresh callback if provided
                                if let Some(ref callback) = on_rebuild_complete_watch {
                                    eprintln!("🔄 Rafraîchissement de l'interface après rebuild");
                                    callback();
                                }

                                // Hide banner
                                rebuild_banner_watch.set_revealed(false);

                                // Clean up
                                let _ = std::fs::remove_file(&status_file_path);
                                let _ = std::fs::remove_file(&script_path);

                                return glib::ControlFlow::Break;
                            }

                            // Stop after 10 minutes (300 checks * 2 seconds)
                            if count > 300 {
                                eprintln!("⚠️ Timeout du watcher de rebuild");
                                rebuild_banner_watch.set_revealed(false);
                                let _ = std::fs::remove_file(&script_path);
                                return glib::ControlFlow::Break;
                            }

                            glib::ControlFlow::Continue
                        });
                    }
                });
            }
            Err(e) => {
                eprintln!("Failed to generate config: {}", e);
                rebuild_error_banner.set_revealed(true);
            }
        }
    }

    pub fn save_config(&self) {
        let disks_widget_clone = self.disks_widget.clone();
        let refresh_callback = Rc::new(move || {
            disks_widget_clone.refresh();
        });

        Self::do_save_config(
            &self.config_file,
            &self.disks,
            &self.hardware_config,
            &self.rebuild_banner,
            &self.rebuild_error_banner,
            &self.must_save,
            Some(refresh_callback),
        );
    }

    pub fn present(&self) {
        self.window.present();
    }

    pub fn gtk_window(&self) -> &adw::ApplicationWindow {
        &self.window
    }

    pub fn refresh_disks_widget(&self) {
        self.disks_widget.refresh();
    }
}
