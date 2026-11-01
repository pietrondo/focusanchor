//! Interfaccia grafica: tema, componenti riusabili e colore.

use eframe::egui;

/// Sfondo generale della finestra.
pub const SFONDO: egui::Color32 = egui::Color32::from_rgb(17, 19, 24);

/// Sfondo delle schede (i riquadri che raccolgono una sezione).
pub const SCHEDA: egui::Color32 = egui::Color32::from_rgb(26, 29, 36);

/// Bordo sottile delle schede.
pub const BORDO: egui::Color32 = egui::Color32::from_rgb(42, 47, 58);

/// Testo principale.
pub const TESTO: egui::Color32 = egui::Color32::from_rgb(232, 234, 240);

/// Testo secondario, piu' tenue.
pub const TENUE: egui::Color32 = egui::Color32::from_rgb(154, 163, 178);

/// Colore d'accento per le azioni principali.
pub const ACCENTO: egui::Color32 = egui::Color32::from_rgb(108, 140, 255);

/// Colore dei punti XP.
pub const VIOLA: egui::Color32 = egui::Color32::from_rgb(180, 140, 255);

/// Colore per gli avvisi che non bloccano.
pub const GIALLO: egui::Color32 = egui::Color32::from_rgb(255, 199, 90);

/// Colore per le conferme e i successi.
pub const VERDE: egui::Color32 = egui::Color32::from_rgb(111, 207, 151);

/// Colore per le sfide anti-sonno.
pub const ARANCIO: egui::Color32 = egui::Color32::from_rgb(255, 159, 90);

/// Applica il tema scuro ad alto contrasto, con testi grandi.
///
/// I caratteri sono volutamente grandi (16 px base, 24 px per i titoli) per
/// ridurre l'affaticamento visivo tipico della dislessia.
pub fn applica_tema(ctx: &egui::Context) {
    let mut stile = (*ctx.style()).clone();
    stile.visuals.dark_mode = true;
    stile.visuals.override_text_color = Some(TESTO);
    stile.visuals.panel_fill = SFONDO;
    stile.visuals.window_fill = SCHEDA;
    stile.visuals.extreme_bg_color = egui::Color32::from_rgb(14, 16, 20);
    stile.visuals.widgets.noninteractive.bg_fill = SCHEDA;
    stile.visuals.widgets.noninteractive.bg_stroke =
        egui::Stroke::new(1.0, BORDO);
    stile.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(38, 43, 53);
    stile.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDO);
    stile.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 60, 76);
    stile.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, ACCENTO);
    stile.visuals.widgets.active.bg_fill = ACCENTO;
    stile.visuals.selection.bg_fill = ACCENTO.gamma_multiply(0.45);
    stile.spacing.item_spacing = egui::vec2(10.0, 10.0);
    stile.spacing.button_padding = egui::vec2(14.0, 8.0);
    stile.spacing.scroll.bar_width = 10.0;
    stile.text_styles.insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    stile
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.0));
    stile
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(16.0));
    stile
        .text_styles
        .insert(egui::TextStyle::Monospace, egui::FontId::monospace(15.0));
    stile
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
    ctx.set_style(stile);
}

/// Larghezza da dare alle schede, gia' al netto della barra di scorrimento.
///
/// Va chiamata FUORI dallo `ScrollArea`, dove la larghezza disponibile e'
/// quella vera della finestra.
pub fn larghezza_schede(ui: &egui::Ui) -> f32 {
    let base = larghezza(ui);
    let calcolata = (base - ui.spacing().scroll.bar_width - 4.0).max(160.0);
    // Mai piu' larga della finestra: un contenuto piu' largo del viewport
    // manda in NaN il calcolo dei click di egui.
    calcolata.min(ui.ctx().screen_rect().width().max(160.0))
}

/// Larghezza disponibile, mai infinita.
///
/// `min_size` con `f32::INFINITY` sembra comodo per i pulsanti a tutta
/// larghezza, ma un infinito nel layout produce `inf - inf = NaN` e fa
/// esplodere il calcolo dei click di egui. Meglio la larghezza vera.
pub fn larghezza(ui: &egui::Ui) -> f32 {
    let w = ui.available_width();
    if w.is_finite() && w > 1.0 {
        w
    } else {
        280.0
    }
}

/// Margine interno delle schede.
const MARGINE_SCHEDA: f32 = 14.0;

/// Riquadro di sezione con titolo, icona e colore d'accento.
///
/// Raccoglie i controlli di un argomento in un blocco visivamente separato:
/// la gerarchia si legge a colpo d'occhio, senza dover leggere il testo.
///
/// `larghezza` va calcolata FUORI dallo `ScrollArea`: dentro, con
/// `auto_shrink` disattivato, la larghezza disponibile puo' essere infinita e
/// un infinito nel layout produce schede larghe a dismisura.
pub fn scheda(
    ui: &mut egui::Ui,
    larghezza: f32,
    icona: &str,
    titolo: &str,
    colore: egui::Color32,
    corpo: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(SCHEDA)
        .stroke(egui::Stroke::new(1.0, BORDO))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(MARGINE_SCHEDA))
        .show(ui, |ui| {
            ui.set_min_width((larghezza - 2.0 * MARGINE_SCHEDA).max(100.0));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(icona).size(17.0));
                ui.label(
                    egui::RichText::new(titolo.to_uppercase())
                        .size(13.0)
                        .strong()
                        .color(colore),
                );
            });
            ui.add_space(6.0);
            corpo(ui);
        });
    ui.add_space(10.0);
}

/// Piccolo badge colorato, per valori brevi accanto a un'etichetta.
pub fn badge(ui: &mut egui::Ui, testo: &str, colore: egui::Color32) {
    egui::Frame::none()
        .fill(colore.gamma_multiply(0.18))
        .stroke(egui::Stroke::new(1.0, colore.gamma_multiply(0.55)))
        .rounding(egui::Rounding::same(20.0))
        .inner_margin(egui::Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(testo).size(13.0).strong().color(colore));
        });
}

/// Barra dei punti XP, del grado e dell'avanzamento verso il grado successivo.
pub fn barra_xp(ui: &mut egui::Ui, stato: &crate::state::StatoApp) {
    ui.horizontal(|ui| {
        badge(ui, &format!("⭐ {} XP", stato.xp.punti), VIOLA);
        ui.label(
            egui::RichText::new(stato.xp.grado())
                .size(15.0)
                .color(TENUE),
        );
        if stato.xp.distrazioni > 0 {
            badge(ui, &format!("⛔ {}", stato.xp.distrazioni), GIALLO);
        }
        if stato.task.completati > 0 {
            badge(ui, &format!("✅ {}", stato.task.completati), VERDE);
        }
    });

    if let Some((soglia, nome)) = stato.xp.prossima_soglia() {
        let precedente = precedente_soglia(&nome);
        let frazione = ((stato.xp.punti - precedente) as f32
            / (soglia - precedente) as f32)
            .clamp(0.0, 1.0);
        ui.add_space(4.0);
        // Il testo va fuori dalla barra: dentro diventa illeggibile quando
        // la barra e' quasi vuota.
        ui.add(
            egui::ProgressBar::new(frazione)
                .desired_height(9.0)
                .rounding(egui::Rounding::same(5.0))
                .fill(VIOLA),
        );
        ui.label(
            egui::RichText::new(format!(
                "{}/{soglia} XP per «{nome}»",
                stato.xp.punti
            ))
            .size(12.0)
            .color(TENUE),
        );
    }
}

/// Punti di partenza del grado indicato, per calcolare l'avanzamento.
fn precedente_soglia(nome: &str) -> i32 {
    match nome {
        "Studioso" => 0,
        "Centurione del Focus" => crate::xp::SOGLIA_STUDIOSO,
        _ => 0,
    }
}

/// Accorcia un testo lungo senza spezzare le parole a meta'.
pub fn accorcia(testo: &str, massimo: usize) -> String {
    if testo.chars().count() <= massimo {
        return testo.to_string();
    }
    let tagliato: String = testo.chars().take(massimo).collect();
    match tagliato.rfind(' ') {
        Some(pos) => format!("{}…", &tagliato[..pos]),
        None => format!("{tagliato}…"),
    }
}

/// Mostra una notifica di conferma in verde.
pub fn notifica(ui: &mut egui::Ui, testo: Option<&str>) {
    if let Some(testo) = testo {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("✅ {testo}")).color(VERDE));
    }
}

/// Mostra un avviso in giallo.
pub fn avviso(ui: &mut egui::Ui, testo: &Option<String>) {
    if let Some(testo) = testo {
        ui.add_space(4.0);
        ui.label(egui::RichText::new(format!("⚠ {testo}")).color(GIALLO));
    }
}

/// Mostra il contenuto di un file in un pannello scorrevole.
pub fn mostra_contenuto(ui: &mut egui::Ui, nome: &str, contenuto: &str) {
    ui.label(egui::RichText::new(nome).strong().color(TENUE));
    if contenuto.trim().is_empty() {
        ui.label(egui::RichText::new("Il file e' vuoto.").color(TENUE));
        return;
    }
    egui::Frame::none()
        .fill(egui::Color32::from_rgb(14, 16, 20))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .max_height(170.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(contenuto)
                            .monospace()
                            .size(13.0)
                            .color(TESTO),
                    );
                });
        });
}
