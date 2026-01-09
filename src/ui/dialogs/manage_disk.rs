use crate::models::{Disk, Partition};
use gettextrs::gettext;
use gtk4::prelude::*;
use gtk4::{Button, Entry, Label, Orientation};
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// List of critical mount points that should be filtered out
const CRITICAL_MOUNT_POINTS: &[&str] = &["/", "/boot", "/boot/efi", "/nix", "/nix/store"];

/// Check if a partition has critical mount points
fn has_critical_mount_point(mount_points: &[String]) -> bool {
    mount_points
        .iter()
        .any(|mp| CRITICAL_MOUNT_POINTS.contains(&mp.as_str()))
}

/// Check if a partition should be filtered out (critical mount points or swap)
fn should_filter_partition(partition: &Partition) -> bool {
    // Filter out partitions with critical mount points
    if has_critical_mount_point(&partition.mount_points) {
        return true;
    }

    // Filter out swap partitions
    if let Some(ref fs_type) = partition.fs_type
        && fs_type == "swap"
    {
        return true;
    }

    false
}

#[allow(dead_code)]
pub struct ManageDiskDialog {
    window: adw::Window,
    content: gtk4::Box,
    disks: Rc<RefCell<Vec<Disk>>>,
    disk_path: std::path::PathBuf,
    on_save_callback: Option<Rc<dyn Fn()>>,
}

impl ManageDiskDialog {
    pub fn new(
        disk: &Disk,
        disks: Rc<RefCell<Vec<Disk>>>,
        on_save_callback: Option<Rc<dyn Fn()>>,
    ) -> Self {
        // Create a proper window
        let window = adw::Window::builder()
            .modal(true)
            .default_width(600)
            .default_height(500)
            .build();

        // Use ToolbarView for proper header
        let toolbar_view = adw::ToolbarView::new();

        let header = adw::HeaderBar::new();
        header.set_title_widget(Some(&gtk4::Label::new(Some(&format!(
            "Gérer {}",
            disk.path.display()
        )))));
        toolbar_view.add_top_bar(&header);

        // Create scrollable content area with partitions
        let scrolled = gtk4::ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        let content = gtk4::Box::new(Orientation::Vertical, 12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // Disk info
        let info_label = Label::new(Some(&format!(
            "Disque: {}\nTaille: {} GB",
            disk.path.display(),
            disk.size / 1_000_000_000
        )));
        info_label.set_halign(gtk4::Align::Start);
        info_label.add_css_class("title-3");
        info_label.set_margin_bottom(12);
        content.append(&info_label);

        // Partitions list (filter out critical partitions and swap)
        let non_critical_partitions: Vec<_> = disk
            .partitions
            .iter()
            .filter(|p| !should_filter_partition(p))
            .collect();

        if non_critical_partitions.is_empty() {
            let no_parts = Label::new(Some(
                "Aucune partition gérable\n(seules les partitions système sont présentes)",
            ));
            no_parts.set_justify(gtk4::Justification::Center);
            content.append(&no_parts);
        } else {
            for partition in non_critical_partitions {
                let part_box =
                    Self::create_partition_row(partition, disks.clone(), on_save_callback.clone());
                content.append(&part_box);

                // Add separator between partitions
                let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                separator.set_margin_top(12);
                separator.set_margin_bottom(12);
                content.append(&separator);
            }
        }

        scrolled.set_child(Some(&content));
        toolbar_view.set_content(Some(&scrolled));
        window.set_content(Some(&toolbar_view));

        Self {
            window,
            content: content.clone(),
            disks: disks.clone(),
            disk_path: disk.path.clone(),
            on_save_callback,
        }
    }

    fn create_partition_row(
        partition: &Partition,
        disks: Rc<RefCell<Vec<Disk>>>,
        on_save_callback: Option<Rc<dyn Fn()>>,
    ) -> gtk4::Box {
        let row = gtk4::Box::new(Orientation::Vertical, 12);

        // Partition info in a card-like box
        let info_box = gtk4::Box::new(Orientation::Vertical, 6);
        info_box.add_css_class("card");
        info_box.set_margin_bottom(6);

        let info = Label::new(Some(&format!(
            "{} - {} {}",
            partition.path.display(),
            partition.fs_type.as_deref().unwrap_or("unknown"),
            if let Some(label) = &partition.label {
                format!("({})", label)
            } else {
                String::new()
            }
        )));
        info.set_halign(gtk4::Align::Start);
        info.set_valign(gtk4::Align::Center);
        info.set_xalign(0.0); // Align text to the left
        info.set_yalign(0.5); // Center text vertically
        info.add_css_class("title-4");
        info.set_margin_top(12);
        info.set_margin_start(12);
        info.set_margin_end(12);
        info.set_margin_bottom(12);
        info_box.append(&info);

        // Add "already mounted" warning if partition has mount points
        if !partition.mount_points.is_empty() {
            let warning_box = gtk4::Box::new(Orientation::Horizontal, 6);
            warning_box.set_margin_start(12);
            warning_box.set_margin_end(12);
            warning_box.set_margin_bottom(8);

            let warning_label = Label::new(Some("⚠️  Partition déjà montée"));
            warning_label.add_css_class("caption");
            warning_label.add_css_class("warning");
            warning_label.set_halign(gtk4::Align::Start);
            warning_box.append(&warning_label);

            info_box.append(&warning_box);
        }

        row.append(&info_box);

        // Mount points section
        let mount_label = Label::new(Some("Points de montage :"));
        mount_label.set_halign(gtk4::Align::Start);
        mount_label.add_css_class("heading");
        mount_label.set_margin_top(12);
        mount_label.set_margin_bottom(6);
        row.append(&mount_label);

        let partition_clone = partition.clone();
        let disks_for_mount = disks.clone();

        // Show existing mount points
        for mount_point in &partition.mount_points {
            let mp_box = gtk4::Box::new(Orientation::Horizontal, 12);
            mp_box.set_margin_start(12);

            let mp_label = Label::new(Some(mount_point));
            mp_label.set_halign(gtk4::Align::Start);
            mp_label.set_hexpand(true);
            mp_box.append(&mp_label);

            let remove_btn = Button::builder()
                .icon_name("user-trash-symbolic")
                .tooltip_text("Supprimer ce point de montage")
                .build();
            remove_btn.add_css_class("flat");
            remove_btn.add_css_class("destructive-action");

            let mount_point_clone = mount_point.clone();
            let partition_path = partition_clone.path.clone();
            let disks_for_remove = disks_for_mount.clone();
            let on_save_callback_for_remove = on_save_callback.clone();

            remove_btn.connect_clicked(move |btn| {
                // Create confirmation dialog
                let dialog = adw::MessageDialog::new(
                    btn.root()
                        .and_then(|r| r.downcast::<gtk4::Window>().ok())
                        .as_ref(),
                    Some(&gettext("Confirm mount point removal")),
                    Some(&format!(
                        "{}\n\n{}",
                        gettext("Do you really want to remove the mount point '%s'?")
                            .replace("%s", &mount_point_clone),
                        gettext(
                            "This action will save the configuration and rebuild the NixOS system."
                        )
                    )),
                );

                dialog.add_response("cancel", &gettext("Cancel"));
                dialog.add_response("confirm", &gettext("Confirm"));
                dialog.set_response_appearance("confirm", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("confirm"));
                dialog.set_close_response("cancel");

                let disks_for_confirm = disks_for_remove.clone();
                let partition_path_for_confirm = partition_path.clone();
                let mount_point_for_confirm = mount_point_clone.clone();
                let on_save_callback_for_confirm = on_save_callback_for_remove.clone();
                let btn_for_confirm = btn.clone();

                dialog.connect_response(None, move |_, response| {
                    if response == "confirm" {
                        eprintln!("✓ Confirmation reçue, suppression du point de montage");

                        let mut removed = false;
                        {
                            let mut disks_mut = disks_for_confirm.borrow_mut();
                            for disk in disks_mut.iter_mut() {
                                for part in disk.partitions.iter_mut() {
                                    if part.path == partition_path_for_confirm {
                                        part.mount_points
                                            .retain(|mp| mp != &mount_point_for_confirm);
                                        removed = true;
                                        eprintln!(
                                            "✓ Point de montage supprimé: {}",
                                            mount_point_for_confirm
                                        );
                                        eprintln!(
                                            "✓ Points de montage restants: {:?}",
                                            part.mount_points
                                        );
                                        break;
                                    }
                                }
                                if removed {
                                    break;
                                }
                            }
                        }

                        // If successfully removed, trigger save callback
                        if removed {
                            eprintln!("📍 Appel de la callback de sauvegarde...");
                            if let Some(ref callback) = on_save_callback_for_confirm {
                                callback();
                            }

                            // Close the manage dialog
                            if let Some(window) = btn_for_confirm
                                .root()
                                .and_then(|r| r.downcast::<gtk4::Window>().ok())
                            {
                                window.close();
                            }
                        }
                    } else {
                        eprintln!("✗ Suppression du point de montage annulée par l'utilisateur");
                    }
                });

                dialog.present();
            });

            mp_box.append(&remove_btn);
            row.append(&mp_box);
        }

        if partition.mount_points.is_empty() {
            let no_mount = Label::new(Some("Aucun point de montage configuré"));
            no_mount.set_halign(gtk4::Align::Start);
            no_mount.set_margin_start(12);
            no_mount.add_css_class("dim-label");
            row.append(&no_mount);
        }

        // Add mount point section
        let add_box = gtk4::Box::new(Orientation::Horizontal, 12);
        add_box.set_margin_top(12);
        add_box.set_margin_start(12);

        // Prefix label
        let prefix_label = Label::new(Some("/media/"));
        prefix_label.add_css_class("title-4");

        let entry = Entry::builder()
            .placeholder_text("nom_du_dossier (ex: data)")
            .hexpand(true)
            .build();

        let add_btn = Button::from_icon_name("list-add-symbolic");
        add_btn.set_tooltip_text(Some("Ajouter ce point de montage"));
        add_btn.add_css_class("circular");
        add_btn.add_css_class("suggested-action");

        let partition_path_for_add = partition.path.clone();
        let disks_for_add = disks.clone();
        let entry_clone = entry.clone();
        let _row_clone = row.clone();

        let on_save_callback_clone = on_save_callback.clone();
        add_btn.connect_clicked(move |btn| {
            let input = entry_clone.text().to_string().trim().to_string();
            if input.is_empty() {
                return;
            }

            // Build mount point: if starts with /, use as-is, otherwise prepend /media/
            let mount_point = if input.starts_with('/') {
                input
            } else {
                format!("/media/{}", input)
            };

            eprintln!("📍 Tentative d'ajout du point de montage: {}", mount_point);
            eprintln!("📍 Pour la partition: {}", partition_path_for_add.display());

            // Create confirmation dialog using MessageDialog (libadwaita 0.7 compatible)
            let dialog = adw::MessageDialog::new(
                btn.root()
                    .and_then(|r| r.downcast::<gtk4::Window>().ok())
                    .as_ref(),
                Some(&gettext("Confirm mount point addition")),
                Some(&format!(
                    "{}\n\n{}",
                    // TRANSLATORS: %s is the mount point path (e.g., /media/data)
                    gettext("Do you really want to add the mount point '%s'?")
                        .replace("%s", &mount_point),
                    gettext(
                        "This action will save the configuration and rebuild the NixOS system."
                    )
                )),
            );

            dialog.add_response("cancel", &gettext("Cancel"));
            dialog.add_response("confirm", &gettext("Confirm"));
            dialog.set_response_appearance("confirm", adw::ResponseAppearance::Suggested);
            dialog.set_default_response(Some("confirm"));
            dialog.set_close_response("cancel");

            let disks_for_confirm = disks_for_add.clone();
            let partition_path_for_confirm = partition_path_for_add.clone();
            let mount_point_for_confirm = mount_point.clone();
            let entry_for_confirm = entry_clone.clone();
            let btn_for_confirm = btn.clone();
            let on_save_callback_for_confirm = on_save_callback_clone.clone();

            dialog.connect_response(None, move |_, response| {
                if response == "confirm" {
                    eprintln!("✓ Confirmation reçue, ajout du point de montage");

                    let mut added = false;
                    {
                        let mut disks_mut = disks_for_confirm.borrow_mut();
                        eprintln!("📍 Nombre de disques dans RefCell: {}", disks_mut.len());

                        for (disk_idx, disk) in disks_mut.iter_mut().enumerate() {
                            eprintln!(
                                "📍 Vérification disque {}: {}",
                                disk_idx,
                                disk.path.display()
                            );
                            for (part_idx, part) in disk.partitions.iter_mut().enumerate() {
                                eprintln!(
                                    "📍   Vérification partition {}: {}",
                                    part_idx,
                                    part.path.display()
                                );
                                if part.path == partition_path_for_confirm {
                                    eprintln!("📍   ✓ Partition trouvée!");
                                    if !part.mount_points.contains(&mount_point_for_confirm) {
                                        part.mount_points.push(mount_point_for_confirm.clone());
                                        entry_for_confirm.set_text("");
                                        added = true;
                                        eprintln!(
                                            "✓ Point de montage ajouté: {}",
                                            mount_point_for_confirm
                                        );
                                        eprintln!(
                                            "✓ Points de montage actuels: {:?}",
                                            part.mount_points
                                        );
                                    } else {
                                        eprintln!("⚠ Point de montage déjà existant");
                                    }
                                    break;
                                }
                            }
                            if added {
                                break;
                            }
                        }

                        if !added {
                            eprintln!(
                                "❌ ERREUR: Partition non trouvée dans la liste des disques!"
                            );
                        }
                    }

                    // If successfully added, trigger save callback
                    if added {
                        eprintln!("📍 Appel de la callback de sauvegarde...");
                        if let Some(ref callback) = on_save_callback_for_confirm {
                            callback();
                        }

                        // Close the manage dialog
                        if let Some(window) = btn_for_confirm
                            .root()
                            .and_then(|r| r.downcast::<gtk4::Window>().ok())
                        {
                            window.close();
                        }
                    }
                } else {
                    eprintln!("✗ Ajout du point de montage annulé par l'utilisateur");
                }
            });

            // Present dialog (MessageDialog presents automatically when created with parent)
            dialog.present();
        });

        // Allow Enter key to add mount point
        let add_btn_clone = add_btn.clone();
        entry.connect_activate(move |_| {
            add_btn_clone.emit_clicked();
        });

        add_box.append(&prefix_label);
        add_box.append(&entry);
        add_box.append(&add_btn);
        row.append(&add_box);

        row
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
