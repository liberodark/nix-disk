use crate::models::Disk;
use crate::ui::dialogs::ManageDiskDialog;
use gtk4::prelude::*;
use gtk4::{Button, Image, Label, Orientation};
use libadwaita as adw;
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
fn should_filter_partition(partition: &crate::models::Partition) -> bool {
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

pub struct DisksWidget {
    container: gtk4::Box,
    disks: Rc<RefCell<Vec<Disk>>>,
    hardware_config: Option<Rc<RefCell<String>>>,
    #[allow(clippy::type_complexity)]
    on_save_callback: Rc<RefCell<Option<Rc<dyn Fn()>>>>,
}

impl Clone for DisksWidget {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            disks: self.disks.clone(),
            hardware_config: self.hardware_config.clone(),
            on_save_callback: self.on_save_callback.clone(),
        }
    }
}

impl DisksWidget {
    #[allow(dead_code)]
    pub fn new(disks: Rc<RefCell<Vec<Disk>>>) -> Self {
        Self::new_with_config(disks, None)
    }

    pub fn new_with_config(
        disks: Rc<RefCell<Vec<Disk>>>,
        hardware_config: Option<Rc<RefCell<String>>>,
    ) -> Self {
        let container = gtk4::Box::new(Orientation::Vertical, 20);
        container.set_vexpand(true);
        container.set_valign(gtk4::Align::Center);
        container.set_halign(gtk4::Align::Center);

        let widget = Self {
            container: container.clone(),
            disks: disks.clone(),
            hardware_config,
            on_save_callback: Rc::new(RefCell::new(None)),
        };

        widget.populate();
        widget
    }

    pub fn set_on_save_callback<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        *self.on_save_callback.borrow_mut() = Some(Rc::new(callback));
    }

    fn populate(&self) {
        // Clear existing children
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        let disks = self.disks.borrow();

        if disks.is_empty() {
            let label = Label::new(Some("Aucun disque trouvé"));
            label.add_css_class("title-2");
            self.container.append(&label);
            return;
        }

        // Filter: show disks with non-critical partitions OR empty disks (no partitions)
        let disks_to_show: Vec<&Disk> = disks
            .iter()
            .filter(|disk| {
                // Show disk if:
                // 1. It has at least one partition that is NOT critical (/, /boot, etc.), OR
                // 2. It has no partitions at all (virgin disk)
                disk.partitions.is_empty()
                    || disk.partitions.iter().any(|p| !should_filter_partition(p))
            })
            .collect();

        if disks_to_show.is_empty() {
            let label = Label::new(Some(
                "Aucun disque gérable disponible\n(seuls les disques système sont présents)",
            ));
            label.add_css_class("title-2");
            label.set_justify(gtk4::Justification::Center);
            self.container.append(&label);
            return;
        }

        // Container for disk cards (horizontal layout)
        let cards_container = gtk4::Box::new(Orientation::Horizontal, 20);
        cards_container.set_halign(gtk4::Align::Center);
        cards_container.set_valign(gtk4::Align::Center);

        for disk in disks_to_show {
            let disk_card = self.create_disk_card(disk);
            cards_container.append(&disk_card);
        }

        self.container.append(&cards_container);
    }

    fn create_disk_card(&self, disk: &Disk) -> gtk4::Box {
        // Card container
        let card = gtk4::Box::new(Orientation::Vertical, 20);
        card.add_css_class("card");
        card.set_width_request(300); // Fixed width for all cards
        // Height will be determined dynamically based on content

        // Disk icon
        let icon = Image::from_icon_name("drive-harddisk");
        icon.set_icon_size(gtk4::IconSize::Large);
        icon.set_pixel_size(64);
        icon.set_margin_top(30);

        // Disk label with name and size
        let size_gb = disk.size / 1_000_000_000;
        let disk_label = Label::new(Some(&format!("{} ({}G)", disk.path.display(), size_gb)));
        disk_label.add_css_class("heading");

        card.append(&icon);
        card.append(&disk_label);

        // Show status/partitions info
        let is_virgin = disk.partitions.is_empty();
        if is_virgin {
            let status_label = Label::new(Some("Disque vierge"));
            status_label.add_css_class("dim-label");
            card.append(&status_label);
        } else {
            // Show non-critical partitions with their status
            let non_critical_partitions: Vec<_> = disk
                .partitions
                .iter()
                .filter(|p| !should_filter_partition(p))
                .collect();

            if !non_critical_partitions.is_empty() {
                let partitions_box = gtk4::Box::new(Orientation::Vertical, 6);
                partitions_box.set_margin_top(8);
                partitions_box.set_margin_start(20);
                partitions_box.set_margin_end(20);

                for (idx, partition) in non_critical_partitions.iter().enumerate() {
                    let part_row = gtk4::Box::new(Orientation::Vertical, 2);

                    // Partition name and filesystem
                    let part_name = partition
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown");
                    let fs_type = partition.fs_type.as_deref().unwrap_or("unknown");

                    let part_info = Label::new(Some(&format!("{} ({})", part_name, fs_type)));
                    part_info.add_css_class("caption");
                    part_info.set_halign(gtk4::Align::Start);
                    part_info.set_valign(gtk4::Align::Center);
                    part_info.set_yalign(0.5);
                    part_row.append(&part_info);

                    // Show mount status
                    if !partition.mount_points.is_empty() {
                        let mount_status = Label::new(Some(&format!(
                            "⚠️  Déjà montée: {}",
                            partition.mount_points.join(", ")
                        )));
                        mount_status.add_css_class("caption");
                        mount_status.add_css_class("warning");
                        mount_status.set_halign(gtk4::Align::Start);
                        part_row.append(&mount_status);
                    } else {
                        let mount_status = Label::new(Some("Non montée"));
                        mount_status.add_css_class("caption");
                        mount_status.add_css_class("dim-label");
                        mount_status.set_halign(gtk4::Align::Start);
                        part_row.append(&mount_status);
                    }

                    partitions_box.append(&part_row);

                    // Add separator between partitions (except for the last one)
                    if idx < non_critical_partitions.len() - 1 {
                        let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
                        sep.set_margin_top(4);
                        sep.set_margin_bottom(4);
                        partitions_box.append(&sep);
                    }
                }

                card.append(&partitions_box);
            }
        }

        // Spacer to push button to bottom
        let spacer = gtk4::Box::new(Orientation::Vertical, 0);
        spacer.set_vexpand(true);
        card.append(&spacer);

        // Manage button (or Format button for virgin disks)
        let manage_button = Button::new();
        let button_content = if is_virgin {
            adw::ButtonContent::builder()
                .icon_name("document-new-symbolic")
                .label("Formater")
                .build()
        } else {
            adw::ButtonContent::builder()
                .icon_name("system-run-symbolic")
                .label("Gérer")
                .build()
        };

        manage_button.set_child(Some(&button_content));
        manage_button.add_css_class("pill");

        if is_virgin {
            manage_button.add_css_class("warning");
        } else {
            manage_button.add_css_class("suggested-action");
        }

        manage_button.set_margin_start(20);
        manage_button.set_margin_end(20);
        manage_button.set_margin_bottom(30);

        // Connect manage/format button
        let disk_clone = disk.clone();
        let disks_rc = self.disks.clone();
        let container_clone = self.container.clone();
        let disks_for_refresh = self.disks.clone();
        let hardware_config_clone = self.hardware_config.clone();
        let on_save_callback_clone = self.on_save_callback.clone();

        manage_button.connect_clicked(move |btn| {
            if let Some(window) = btn.root().and_then(|r| r.downcast::<gtk4::Window>().ok()) {
                if disk_clone.partitions.is_empty() {
                    // Show format dialog
                    use crate::ui::dialogs::FormatDiskDialog;

                    // Create refresh callback
                    let container_refresh = container_clone.clone();
                    let disks_refresh = disks_for_refresh.clone();
                    let hardware_config_for_refresh = hardware_config_clone.clone();
                    let on_save_callback_for_refresh = on_save_callback_clone.clone();

                    let refresh_callback = move || {
                        eprintln!("🔄 Rafraîchissement après formatage...");

                        // Reload disks from system with hardware config
                        use crate::utils::get_disks;
                        let config_ref = hardware_config_for_refresh
                            .as_ref()
                            .map(|c| c.borrow().clone());
                        let config_str = config_ref.as_deref();

                        if let Ok(new_disks) = get_disks(config_str) {
                            *disks_refresh.borrow_mut() = new_disks;
                        }

                        // Clear and repopulate container
                        while let Some(child) = container_refresh.first_child() {
                            container_refresh.remove(&child);
                        }

                        // Recreate the widget to repopulate
                        let temp_widget = Self::new_with_config(
                            disks_refresh.clone(),
                            hardware_config_for_refresh.clone(),
                        );

                        // Reapply the save callback to the new widget
                        if let Some(callback) = on_save_callback_for_refresh.borrow().as_ref() {
                            let callback_clone = callback.clone();
                            temp_widget.set_on_save_callback(move || callback_clone());
                        }

                        while let Some(child) = temp_widget.container.first_child() {
                            temp_widget.container.remove(&child);
                            container_refresh.append(&child);
                        }
                    };

                    let dialog =
                        FormatDiskDialog::new(&disk_clone, disks_rc.clone(), refresh_callback);
                    dialog.present(Some(&window));
                } else {
                    // Show manage dialog
                    let callback = on_save_callback_clone.borrow().clone();
                    let dialog = ManageDiskDialog::new(&disk_clone, disks_rc.clone(), callback);
                    dialog.present(Some(&window));
                }
            }
        });

        card.append(&manage_button);

        card
    }

    pub fn widget(&self) -> gtk4::Box {
        self.container.clone()
    }

    pub fn refresh(&self) {
        eprintln!("🔄 Rafraîchissement du widget des disques");
        eprintln!(
            "📊 Nombre de disques actuels: {}",
            self.disks.borrow().len()
        );

        self.populate();

        // Force redraw of the container and its parents
        self.container.queue_resize();
        self.container.queue_draw();

        // Also force redraw of parent widgets up the hierarchy
        if let Some(parent) = self.container.parent() {
            parent.queue_resize();
            parent.queue_draw();
            eprintln!("🔄 Redessin du parent demandé");

            // Go up one more level if possible
            if let Some(grandparent) = parent.parent() {
                grandparent.queue_resize();
                grandparent.queue_draw();
                eprintln!("🔄 Redessin du grand-parent demandé");
            }
        }

        eprintln!("✅ Widget des disques rafraîchi");
    }

    /// Returns the number of disks that will be shown in the UI
    pub fn count_visible_disks(&self) -> usize {
        let disks = self.disks.borrow();

        // Filter: show disks with non-critical partitions OR empty disks (no partitions)
        disks
            .iter()
            .filter(|disk| {
                disk.partitions.is_empty()
                    || disk.partitions.iter().any(|p| !should_filter_partition(p))
            })
            .count()
    }

    /// Returns the maximum height needed for disk cards based on partition counts
    pub fn get_max_card_height(&self) -> i32 {
        let disks = self.disks.borrow();

        // Base height for empty card: icon(64) + margins(30+20) + label + spacer + button + bottom margin(30) ≈ 250px
        let base_height = 250;

        // Calculate height needed for the disk with most partitions
        let max_partitions_height = disks
            .iter()
            .filter(|disk| {
                disk.partitions.is_empty()
                    || disk.partitions.iter().any(|p| !should_filter_partition(p))
            })
            .map(|disk| {
                let non_critical_count = disk
                    .partitions
                    .iter()
                    .filter(|p| !should_filter_partition(p))
                    .count();

                if non_critical_count == 0 {
                    0
                } else {
                    // Each partition: name + status + margins ≈ 60px
                    // Plus separator between partitions ≈ 10px
                    // Plus container margins (top 8 + start/end 20+20) ≈ 50px
                    let partition_height = non_critical_count as i32 * 60;
                    let separator_height = (non_critical_count.saturating_sub(1)) as i32 * 10;
                    partition_height + separator_height + 50
                }
            })
            .max()
            .unwrap_or(0);

        base_height + max_partitions_height
    }
}
