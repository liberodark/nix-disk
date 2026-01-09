use gtk4::prelude::*;
use libadwaita as adw;
use libadwaita::prelude::*;

pub struct WelcomeDialog {
    dialog: adw::MessageDialog,
}

impl WelcomeDialog {
    pub fn new() -> Self {
        let dialog = adw::MessageDialog::new(
            None::<&gtk4::Window>,
            Some("Bienvenue dans Nix-disk"),
            Some("Gérez vos points de montage de disques facilement.\n\nCette application vous aidera à configurer les montages de systèmes de fichiers dans votre configuration matérielle NixOS."),
        );

        dialog.add_response("start", "Commencer");
        dialog.set_response_appearance("start", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("start"));
        dialog.set_close_response("start");

        Self { dialog }
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
