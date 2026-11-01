//! Comandi arrivati dal pannello FocusAnchor dentro Zotero.
//!
//! Il punto di partenza dello studio e' Zotero: aprire una seconda finestra
//! solo per premere "avvia" e' l'attrito che fa saltare la sessione. Il
//! plugin scrive un comando, l'app lo esegue e risponde con un battito che
//! il pannello mostra.

use eframe::egui;

use crate::comando;
use crate::ui_esami;

impl crate::AppFocus {
    /// Battito per l'estensione Zotero e comandi in arrivo dal suo pannello.
    ///
    /// Il battito dice al plugin che l'app e' viva e cosa sta facendo; il
    /// comando e' un file che si consuma, cosi' un riavvio non lo ripete.
    pub(crate) fn comandi(&mut self, ctx: &egui::Context) {
        if self.ultimo_battito.elapsed().as_secs() >= 2 {
            self.ultimo_battito = std::time::Instant::now();
            self.stato.sincronizza_memoria();
            comando::battito(&self.stato);
        }
        let Some(comando) = comando::leggi(&self.stato.config.cartella_zotero) else {
            return;
        };
        if self.ultimo_comando.as_deref() == Some(comando.id.as_str()) {
            return;
        }
        self.ultimo_comando = Some(comando.id.clone());
        self.stato.ultimo_comando = comando.comando.clone();
        comando::consuma(&self.stato.config.cartella_zotero);
        self.esegui_comando(comando, ctx);
    }

    /// Esegue un comando: e' la sessione comandata da Zotero.
    pub(crate) fn esegui_comando(&mut self, comando: comando::Comando, ctx: &egui::Context) {
        match comando.comando.as_str() {
            "avvia" => self.comando_avvia(&comando.esame, ctx),
            "pausa" => {
                self.stato.metti_in_pausa();
                self.stato.aggiungi_notifica("Pausa di 5 minuti.");
            }
            "riprendi" => self.stato.riprendi_sessione(),
            "ferma" => self.stato.termina_sessione(),
            "ripasso" => {
                let _ = ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            }
            altro => self.stato.avviso = Some(format!("Comando sconosciuto: {altro}")),
        }
    }

    /// Avvia la sessione sull'esame che Zotero ha davanti.
    pub(crate) fn comando_avvia(&mut self, esame: &str, ctx: &egui::Context) {
        if self.stato.sessione.is_some() {
            return;
        }
        let trovata = self
            .stato
            .collezioni
            .iter()
            .find(|collezione| collezione.nome == esame)
            .map(|collezione| collezione.id);
        let Some(id) = trovata else {
            // Senza collezioni caricate non si sa quali testi ruotare: meglio
            // dirlo che avviare una sessione vuota.
            self.stato.avviso = Some(
                "FocusAnchor non ha ancora gli esami: aprilo una volta e premi \
                 «Carica esami da Zotero»."
                    .into(),
            );
            return;
        };
        ui_esami::cambia_esame(&mut self.stato, Some(id));
        ui_esami::avvia_sessione(&mut self.stato, &mut self.audio, &self.mittente);
        let _ = ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }
}
