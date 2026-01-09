use crate::models::{Disk, Partition};
use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

pub struct MissingPartitionsDialog {
    dialog: adw::MessageDialog,
}

impl MissingPartitionsDialog {
    pub fn new(missing: &[Partition], disks: Rc<RefCell<Vec<Disk>>>) -> Self {
        let mut message = String::from("Les partitions suivantes sont configurées mais n'existent plus :\n\n");

        for partition in missing {
            message.push_str(&format!(
                "• {} (monté sur : {})\n",
                partition.path.display(),
                partition.mount_points.join(", ")
            ));
        }

        message.push_str("\nVoulez-vous les retirer de la configuration ?");

        let dialog = adw::MessageDialog::new(
            None::<&gtk4::Window>,
            Some("Partitions Manquantes Détectées"),
            Some(&message),
        );

        dialog.add_response("cancel", "Garder dans la Configuration");
        dialog.add_response("remove", "Retirer");
        dialog.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");

        // Clone missing partitions for the closure
        let missing_vec = missing.to_vec();
        dialog.connect_response(None, move |_, response| {
            if response == "remove" {
                Self::remove_missing_partitions(&missing_vec, &disks);
            }
        });

        Self { dialog }
    }

    fn remove_missing_partitions(missing: &[Partition], disks: &Rc<RefCell<Vec<Disk>>>) {
        let mut disks_mut = disks.borrow_mut();

        for disk in disks_mut.iter_mut() {
            disk.partitions.retain(|p| {
                !missing.iter().any(|m| m.path == p.path)
            });
        }
    }

    pub fn present(&self, parent: Option<&impl IsA<gtk4::Widget>>) {
        if let Some(p) = parent {
            if let Some(window) = p.dynamic_cast_ref::<gtk4::Window>() {
                self.dialog.set_transient_for(Some(window));
            }
        }
        self.dialog.present();
    }
}
