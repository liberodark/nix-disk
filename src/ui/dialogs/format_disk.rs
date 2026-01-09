use crate::models::Disk;
use gtk4::prelude::*;
use gtk4::{Button, Entry, Label, Orientation};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::process::Command;
use std::rc::Rc;

pub struct FormatDiskDialog {
    window: adw::Window,
}

impl FormatDiskDialog {
    pub fn new<F>(disk: &Disk, disks: Rc<RefCell<Vec<Disk>>>, on_complete: F) -> Self
    where
        F: Fn() + 'static,
    {
        // Create a proper window
        let window = adw::Window::builder()
            .modal(true)
            .default_width(500)
            .default_height(400)
            .build();

        // Use ToolbarView for proper header
        let toolbar_view = adw::ToolbarView::new();

        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&Label::new(Some("Formater le disque"))));
        toolbar_view.add_top_bar(&header);

        // Create content area
        let content = gtk4::Box::new(Orientation::Vertical, 24);
        content.set_margin_top(24);
        content.set_margin_bottom(24);
        content.set_margin_start(24);
        content.set_margin_end(24);
        content.set_valign(gtk4::Align::Center);

        // Warning icon and message
        let warning_box = gtk4::Box::new(Orientation::Vertical, 12);
        warning_box.set_halign(gtk4::Align::Center);

        let warning_icon = Label::new(Some("⚠️"));
        warning_icon.add_css_class("title-1");
        warning_box.append(&warning_icon);

        let warning_title = Label::new(Some("ATTENTION"));
        warning_title.add_css_class("title-2");
        warning_box.append(&warning_title);

        content.append(&warning_box);

        // Disk information
        let disk_info = Label::new(Some(&format!(
            "Vous êtes sur le point de formater le disque :\n\n{} ({} GB)\n\nToutes les données seront DÉFINITIVEMENT PERDUES !\n\nLe disque sera formaté en ext4 et une partition unique sera créée.",
            disk.path.display(),
            disk.size / 1_000_000_000
        )));
        disk_info.set_wrap(true);
        disk_info.set_justify(gtk4::Justification::Center);
        content.append(&disk_info);

        // Volume name entry section
        let entry_box = gtk4::Box::new(Orientation::Vertical, 12);
        entry_box.set_margin_top(12);

        let entry_label = Label::new(Some("Nom du volume :"));
        entry_label.set_halign(gtk4::Align::Start);
        entry_label.add_css_class("heading");
        entry_box.append(&entry_label);

        let volume_entry = Entry::builder()
            .placeholder_text("Ex: data, backup, storage...")
            .hexpand(true)
            .max_length(16) // ext4 label limit
            .build();

        // Set a default volume name based on disk size or path
        let default_name = format!("disk_{}", disk.size / 1_000_000_000);
        volume_entry.set_text(&default_name);

        entry_box.append(&volume_entry);

        let hint_label = Label::new(Some("(Maximum 16 caractères)"));
        hint_label.set_halign(gtk4::Align::Start);
        hint_label.add_css_class("dim-label");
        hint_label.add_css_class("caption");
        entry_box.append(&hint_label);

        content.append(&entry_box);

        // Buttons
        let button_box = gtk4::Box::new(Orientation::Horizontal, 12);
        button_box.set_halign(gtk4::Align::Center);
        button_box.set_margin_top(24);

        let cancel_button = Button::builder().label("Annuler").build();
        cancel_button.add_css_class("pill");

        let format_button = Button::builder().label("Formater le disque").build();
        format_button.add_css_class("pill");
        format_button.add_css_class("destructive-action");

        button_box.append(&cancel_button);
        button_box.append(&format_button);
        content.append(&button_box);

        toolbar_view.set_content(Some(&content));
        window.set_content(Some(&toolbar_view));

        // Handle cancel button
        let window_clone = window.clone();
        cancel_button.connect_clicked(move |_| {
            window_clone.close();
        });

        // Handle format button
        let disk_path = disk.path.clone();
        let disks_clone = disks.clone();
        let on_complete = Rc::new(on_complete);
        let window_clone = window.clone();
        let volume_entry_clone = volume_entry.clone();

        format_button.connect_clicked(move |_| {
            let volume_name = volume_entry_clone.text().to_string().trim().to_string();

            if volume_name.is_empty() {
                // Show error if volume name is empty
                let error_dialog = adw::MessageDialog::new(
                    Some(&window_clone),
                    Some("Nom de volume requis"),
                    Some("Veuillez entrer un nom pour le volume."),
                );
                error_dialog.add_response("ok", "OK");
                error_dialog.set_default_response(Some("ok"));
                error_dialog.present();
                return;
            }

            eprintln!(
                "💾 Formatage du disque {} avec le nom de volume '{}'...",
                disk_path.display(),
                volume_name
            );
            Self::format_disk(
                &disk_path,
                &volume_name,
                &disks_clone,
                &window_clone,
                on_complete.clone(),
            );
            window_clone.close();
        });

        // Allow Enter key to trigger format
        let format_button_clone = format_button.clone();
        volume_entry.connect_activate(move |_| {
            format_button_clone.emit_clicked();
        });

        Self { window }
    }

    fn format_disk<F>(
        disk_path: &std::path::Path,
        volume_name: &str,
        disks: &Rc<RefCell<Vec<Disk>>>,
        parent_window: &adw::Window,
        on_complete: Rc<F>,
    ) where
        F: Fn() + 'static,
    {
        let disk_str = disk_path.to_string_lossy().to_string();
        let volume_name = volume_name.to_string();

        eprintln!("💾 Formatage du disque {}...", disk_str);

        // Get timestamp for unique file names
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Get absolute paths to tools from environment (set by Nix wrapper)
        let parted_bin = std::env::var("PARTED_BIN").unwrap_or_else(|_| "parted".to_string());
        let mkfs_ext4_bin =
            std::env::var("MKFS_EXT4_BIN").unwrap_or_else(|_| "mkfs.ext4".to_string());

        // Get current user info BEFORE launching terminal
        // When running with pkexec, PKEXEC_UID contains the real user's UID
        let uid = std::env::var("PKEXEC_UID")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .or_else(|| {
                // Fallback: try to get UID from current process
                std::process::Command::new("id")
                    .arg("-u")
                    .output()
                    .ok()
                    .and_then(|output| String::from_utf8(output.stdout).ok())
                    .and_then(|s| s.trim().to_string().parse::<u32>().ok())
            })
            .unwrap_or(1000);

        // Get username from UID
        let current_user = std::process::Command::new("id")
            .arg("-nu")
            .arg(uid.to_string())
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "user".to_string());

        // Get GID from UID
        let gid = std::process::Command::new("id")
            .arg("-g")
            .arg(uid.to_string())
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .and_then(|s| s.trim().to_string().parse::<u32>().ok())
            .unwrap_or(100);

        eprintln!(
            "📍 Utilisateur détecté: {} (uid={}, gid={})",
            current_user, uid, gid
        );

        // Create the format script with sudo integrated
        let format_script = format!(
            r#"#!/usr/bin/env bash

echo "======================================"
echo "  FORMATAGE DE DISQUE"
echo "======================================"
echo ""
echo "Formatage du disque: {}"
echo ""

# Function to run commands with sudo while preserving environment
run_sudo() {{
    sudo -E "$@"
}}

echo "Étape 1: Création de la table de partition GPT..."
run_sudo {} -s {} mklabel gpt || exit 1
echo "✓ Table GPT créée"
echo ""

echo "Étape 2: Création de la partition..."
run_sudo {} -s {} mkpart primary ext4 0% 100% || exit 1
echo "✓ Partition créée"
echo ""

echo "Étape 3: Synchronisation..."
sleep 2
sync
echo "✓ Synchronisé"
echo ""

echo "Étape 4: Détection de la partition..."
PARTITION="{}1"
if [ -e "$PARTITION" ]; then
    echo "✓ Partition trouvée: $PARTITION"
else
    PARTITION="{}p1"
    if [ -e "$PARTITION" ]; then
        echo "✓ Partition trouvée: $PARTITION"
    else
        echo "❌ ERREUR: Partition non trouvée"
        exit 1
    fi
fi
echo ""

echo "Étape 5: Formatage en ext4 avec le nom de volume '{}'..."
run_sudo {} -F -L "{}" "$PARTITION" || exit 1
echo "✓ Système de fichiers créé"
echo ""

echo "Étape 6: Configuration des permissions..."
# Mount temporarily to set permissions
TEMP_MOUNT="/tmp/nix_disk_mount_{}"
run_sudo mkdir -p "$TEMP_MOUNT" || exit 1
run_sudo mount "$PARTITION" "$TEMP_MOUNT" || exit 1

# Use the UID and GID provided by the application (captured before terminal launch)
REAL_USER="{}"
REAL_UID={}
REAL_GID={}

echo "Configuration des permissions pour l'utilisateur: $REAL_USER (uid=$REAL_UID, gid=$REAL_GID)"
run_sudo chown "$REAL_UID:$REAL_GID" "$TEMP_MOUNT" || exit 1
run_sudo chmod 755 "$TEMP_MOUNT" || exit 1

# Unmount
run_sudo umount "$TEMP_MOUNT" || exit 1
run_sudo rmdir "$TEMP_MOUNT" || exit 1
echo "✓ Permissions configurées"
echo ""

echo "======================================"
echo "  ✅ FORMATAGE TERMINÉ AVEC SUCCÈS"
echo "======================================"
echo ""

# Signal completion to the app
touch /tmp/nix_disk_manager_format_{}.done

echo "Appuyez sur Entrée ou fermez cette fenêtre..."
read -t 300 || true
"#,
            disk_str,
            parted_bin,
            disk_str,
            parted_bin,
            disk_str,
            disk_str,
            disk_str,
            volume_name,
            mkfs_ext4_bin,
            volume_name,
            timestamp,
            current_user,
            uid,
            gid,
            timestamp
        );

        // Write script to temp file with timestamp to avoid conflicts
        let script_path = format!("/tmp/nix_disk_manager_format_{}.sh", timestamp);
        let status_file = format!("/tmp/nix_disk_manager_format_{}.done", timestamp);

        if let Err(e) = std::fs::write(&script_path, format_script) {
            eprintln!("❌ Erreur: impossible d'écrire le script: {}", e);
            Self::show_error_dialog(parent_window, &format!("Erreur: {}", e));
            return;
        }

        // Make script executable
        if let Err(e) = Command::new("chmod").arg("+x").arg(&script_path).status() {
            eprintln!("❌ Erreur chmod: {}", e);
            Self::show_error_dialog(parent_window, &format!("Erreur: {}", e));
            return;
        }

        // Just execute the script directly - no wrapper needed
        // The script itself will handle sudo prompts

        // On NixOS, we need to use the terminal approach as pkexec might not work well
        eprintln!("💾 Ouverture d'un terminal pour le formatage...");

        let _script_cmd = format!("bash {}", script_path);

        // Try different terminals
        let terminals: Vec<(&str, Vec<&str>)> = vec![
            ("kgx", vec!["--", &script_path]), // GNOME Console
            ("gnome-terminal", vec!["--", &script_path]),
            ("konsole", vec!["-e", &script_path]),
            ("xfce4-terminal", vec!["-e", &script_path]),
            ("alacritty", vec!["-e", &script_path]),
            ("kitty", vec![&script_path]),
            ("xterm", vec!["-e", &script_path]),
        ];

        let mut terminal_opened = false;
        for (term, args) in terminals {
            eprintln!("💾 Tentative avec {}...", term);
            // Use spawn instead of status - we just want to launch the terminal, not wait for it
            if Command::new(term).args(&args).spawn().is_ok() {
                eprintln!("✅ Terminal {} ouvert avec succès", term);
                terminal_opened = true;
                break;
            }
        }

        // Don't show a dialog - the terminal has all the info
        // (Dialogs always go behind the terminal window anyway)
        if !terminal_opened {
            eprintln!("❌ Aucun terminal trouvé");
            Self::show_error_dialog(
                parent_window,
                "Impossible d'ouvrir un terminal. Installez gnome-console (kgx) ou gnome-terminal.",
            );
            // Clean up if we couldn't open a terminal
            let _ = std::fs::remove_file(&script_path);
        } else {
            // Start watching for completion
            use gtk4::glib;
            let _disks_watch = disks.clone();
            let status_file_watch = status_file.clone();
            let script_path_watch = script_path.clone();
            let check_count = Rc::new(RefCell::new(0u32));

            glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
                *check_count.borrow_mut() += 1;
                let count = *check_count.borrow();

                // Check if status file exists
                if std::path::Path::new(&status_file_watch).exists() {
                    eprintln!("✅ Formatage terminé détecté!");

                    // Call completion callback (which will reload disks with config)
                    on_complete();

                    // Clean up
                    let _ = std::fs::remove_file(&status_file_watch);
                    let _ = std::fs::remove_file(&script_path_watch);

                    // Stop watching
                    return glib::ControlFlow::Break;
                }

                // Stop after 5 minutes (150 checks * 2 seconds)
                if count > 150 {
                    eprintln!("⚠️ Timeout du watcher de formatage");
                    let _ = std::fs::remove_file(&script_path_watch);
                    return glib::ControlFlow::Break;
                }

                // Continue watching
                glib::ControlFlow::Continue
            });
        }
    }

    #[allow(dead_code)]
    fn show_success_dialog(parent: &adw::Window, disk: &str) {
        let success_dialog = adw::MessageDialog::new(
            Some(parent),
            Some("Formatage réussi"),
            Some(&format!(
                "Le disque {} a été formaté avec succès en ext4.\n\nVous pouvez maintenant le monter.",
                disk
            )),
        );
        success_dialog.add_response("ok", "OK");
        success_dialog.set_default_response(Some("ok"));
        success_dialog.set_close_response("ok");
        success_dialog.present();
    }

    fn show_error_dialog(parent: &adw::Window, error: &str) {
        let error_dialog = adw::MessageDialog::new(
            Some(parent),
            Some("Erreur de formatage"),
            Some(&format!("Le formatage a échoué :\n\n{}", error)),
        );
        error_dialog.add_response("ok", "OK");
        error_dialog.set_default_response(Some("ok"));
        error_dialog.set_close_response("ok");
        error_dialog.present();
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk4::Widget>>) {
        if let Some(p) = parent
            && let Some(window) = p.dynamic_cast_ref::<gtk4::Window>()
        {
            self.window.set_transient_for(Some(window));
        }
        self.window.present();
    }
}
