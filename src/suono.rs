//! Sorgenti audio procedurali: nessun file esterno, tutto generato via codice.
//!
//! - `RumoreMarrone`: maschera i pensieri intrusivi con un suono caldo.
//! - `Binaurale40`: batte binaurale a 40 Hz (sinistro 200 Hz, destro 240 Hz)
//!   per contrastare la sonnolenza da sotto-stimolazione.
//! - `Beep`: suono breve di sveglia per richiamare l'attenzione.

use std::time::Duration;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rodio::Source;

/// Frequenza di campionamento di tutti i sorgenti.
pub const FREQUENZA: u32 = 44_100;

/// Numero di canali: stereo, necessario per il binaural.
pub const CANALI: u16 = 2;

/// Frequenza del tono sinistro nel binaural.
pub const TONO_SINISTRO: f32 = 200.0;

/// Frequenza del tono destro nel binaural.
pub const TONO_DESTRO: f32 = 240.0;

/// Ampiezza dei toni puri nel binaurale.
const AMPIEZZA_TONO: f32 = 0.30;

/// Sorgente audio di rumore marrone, generata in tempo reale.
///
/// Il rumore bianco viene integrato con un filtro leaky integrator, che
/// sposta l'energia verso le frequenze basse: il risultato e' un suono
/// caldo e profondo, simile a una cascata lontana.
pub struct RumoreMarrone {
    /// Generatore di numeri casuali.
    rng: StdRng,
    /// Ultimo valore dell'integratore (memoria del filtro).
    ultimo: f32,
}

impl RumoreMarrone {
    /// Crea un generatore di rumore marrone.
    pub fn nuovo() -> Self {
        Self {
            rng: StdRng::from_entropy(),
            ultimo: 0.0,
        }
    }
}

impl Iterator for RumoreMarrone {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        // Rumore bianco uniforme in [-1, 1].
        let bianco: f32 = self.rng.gen_range(-1.0..1.0);
        // Integratore leaky: attenua le frequenze alte, esalta le basse.
        self.ultimo = (self.ultimo + 0.02 * bianco) / 1.02;
        // Normalizzazione per compensare l'attenuazione del filtro.
        Some(self.ultimo * 3.5)
    }
}

impl Source for RumoreMarrone {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CANALI
    }

    fn sample_rate(&self) -> u32 {
        FREQUENZA
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Sorgente binaurale a 40 Hz: due toni, uno per orecchio.
///
/// L'orecchio sinistro riceve un tono puro a 200 Hz e quello destro a 240 Hz;
/// la differenza fra le due frequenze produce il battimento percepito di 40 Hz.
pub struct Binaurale40 {
    /// Fase corrente del tono sinistro.
    fase_sinistra: f32,
    /// Fase corrente del tono destro.
    fase_destra: f32,
    /// Incremento di fase per campione del tono sinistro.
    passo_sinistro: f32,
    /// Incremento di fase per campione del tono destro.
    passo_destro: f32,
    /// `true` se il prossimo campione va sul canale sinistro.
    canale_sinistro: bool,
}

impl Binaurale40 {
    /// Crea il generatore binaurale a 40 Hz.
    pub fn nuovo() -> Self {
        Self::nuovo_con_differenza(TONO_DESTRO - TONO_SINISTRO)
    }

    /// Crea il binaurale con una differenza di frequenza esplicita.
    pub fn nuovo_con_differenza(differenza_hz: f32) -> Self {
        let frequenza = FREQUENZA as f32;
        Self {
            fase_sinistra: 0.0,
            fase_destra: 0.0,
            passo_sinistro: 2.0 * std::f32::consts::PI * TONO_SINISTRO / frequenza,
            passo_destro: 2.0 * std::f32::consts::PI * (TONO_SINISTRO + differenza_hz) / frequenza,
            canale_sinistro: true,
        }
    }
}

impl Iterator for Binaurale40 {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let campione = if self.canale_sinistro {
            let valore = self.fase_sinistra.sin() * AMPIEZZA_TONO;
            self.fase_sinistra += self.passo_sinistro;
            if self.fase_sinistra > 2.0 * std::f32::consts::PI {
                self.fase_sinistra -= 2.0 * std::f32::consts::PI;
            }
            valore
        } else {
            let valore = self.fase_destra.sin() * AMPIEZZA_TONO;
            self.fase_destra += self.passo_destro;
            if self.fase_destra > 2.0 * std::f32::consts::PI {
                self.fase_destra -= 2.0 * std::f32::consts::PI;
            }
            valore
        };
        self.canale_sinistro = !self.canale_sinistro;
        Some(campione)
    }
}

impl Source for Binaurale40 {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        CANALI
    }

    fn sample_rate(&self) -> u32 {
        FREQUENZA
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Sorgente di un breve beep, per richiamare l'attenzione.
pub struct Beep {
    /// Fase corrente dell'onda.
    fase: f32,
    /// Incremento di fase per campione.
    passo: f32,
    /// Campioni ancora da emettere.
    rimanenti: usize,
}

impl Beep {
    /// Crea un beep della durata indicata.
    pub fn nuovo(durata: Duration, frequenza: f32) -> Self {
        let campioni = (durata.as_secs_f32() * FREQUENZA as f32) as usize;
        Self {
            fase: 0.0,
            passo: 2.0 * std::f32::consts::PI * frequenza / FREQUENZA as f32,
            rimanenti: campioni,
        }
    }
}

impl Iterator for Beep {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.rimanenti == 0 {
            return None;
        }
        self.rimanenti -= 1;
        let valore = self.fase.sin() * 0.35;
        self.fase += self.passo;
        Some(valore)
    }
}

impl Source for Beep {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.rimanenti)
    }

    fn channels(&self) -> u16 {
        CANALI
    }

    fn sample_rate(&self) -> u32 {
        FREQUENZA
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(
            self.rimanenti as f32 / FREQUENZA as f32,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_binaural_alterna_i_due_canali() {
        let mut sorgente = Binaurale40::nuovo();
        let sinistro_iniziale = sorgente.next().unwrap();
        let destro_iniziale = sorgente.next().unwrap();
        let sinistro_successivo = sorgente.next().unwrap();
        let destro_successivo = sorgente.next().unwrap();

        assert_eq!(sinistro_iniziale, 0.0);
        assert_eq!(destro_iniziale, 0.0);
        assert_ne!(sinistro_successivo, destro_successivo);
    }

    #[test]
    fn il_beep_termina_dopo_la_durata_prevista() {
        let mut beep = Beep::nuovo(Duration::from_millis(50), 880.0);
        let mut campioni = 0;
        while beep.next().is_some() {
            campioni += 1;
        }
        assert!(campioni > 1000 && campioni < 3000);
    }

    #[test]
    fn il_rumore_marrone_restituisce_sempre_valori() {
        let mut rumore = RumoreMarrone::nuovo();
        for _ in 0..100 {
            assert!(rumore.next().is_some());
        }
    }

    #[test]
    fn il_battimento_binaurale_e_di_40_hertz() {
        // La differenza fra i due toni e' la frequenza del battimento percepito.
        assert_eq!(TONO_DESTRO - TONO_SINISTRO, 40.0);
    }

    #[test]
    fn i_due_canali_del_binaural_hanno_frequenze_diverse() {
        // Misura gli attraversamenti dello zero separando i due canali: il
        // destro deve oscillare piu' del sinistro.
        //
        // Un secondo di audio stereo contiene `FREQUENZA` frame, cioe' due
        // campioni per frame: uno per ciascun orecchio.
        let mut sorgente = Binaurale40::nuovo();
        let campioni = FREQUENZA as usize * CANALI as usize;
        let mut sinistra = 0usize;
        let mut destra = 0usize;
        let mut segno_s = 0i8;
        let mut segno_d = 0i8;
        let mut fase_s = 0.0_f32;
        let mut fase_d = 0.0_f32;
        let passo_s = 2.0 * std::f32::consts::PI * TONO_SINISTRO / FREQUENZA as f32;
        let passo_d = 2.0 * std::f32::consts::PI * TONO_DESTRO / FREQUENZA as f32;

        for indice in 0..campioni {
            let campione = sorgente.next().unwrap();
            let atteso = if indice % 2 == 0 {
                let atteso = fase_s.sin() * AMPIEZZA_TONO;
                fase_s += passo_s;
                if fase_s > 2.0 * std::f32::consts::PI {
                    fase_s -= 2.0 * std::f32::consts::PI;
                }
                atteso
            } else {
                let atteso = fase_d.sin() * AMPIEZZA_TONO;
                fase_d += passo_d;
                if fase_d > 2.0 * std::f32::consts::PI {
                    fase_d -= 2.0 * std::f32::consts::PI;
                }
                atteso
            };
            assert!(
                (campione - atteso).abs() < 0.000001,
                "campione {indice} non corrisponde al tono puro: {campione} != {atteso}"
            );
            let segno = if campione >= 0.0 { 1 } else { -1 };
            if indice % 2 == 0 {
                if segno_s != 0 && segno != segno_s {
                    sinistra += 1;
                }
                segno_s = segno;
            } else {
                if segno_d != 0 && segno != segno_d {
                    destra += 1;
                }
                segno_d = segno;
            }
        }

        // Zero crossings occur twice per sine-wave cycle.
        let frequenza_sinistra = sinistra as f32 / 2.0;
        let frequenza_destra = destra as f32 / 2.0;
        assert!(
            (195.0..=205.0).contains(&frequenza_sinistra),
            "il canale sinistro deve stare vicino a 200 Hz ({} Hz)",
            frequenza_sinistra
        );
        assert!(
            (235.0..=245.0).contains(&frequenza_destra),
            "il canale destro deve stare vicino a 240 Hz ({} Hz)",
            frequenza_destra
        );
        assert!(
            ((frequenza_destra - frequenza_sinistra) - 40.0).abs() <= 5.0,
            "il battimento deve essere vicino a 40 Hz, non {} Hz",
            frequenza_destra - frequenza_sinistra
        );
    }
}
