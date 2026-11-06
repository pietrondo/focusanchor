//! Pioggia procedurale: fruscio filtrato piu' gocce sparse.
//!
//! Il rumore marrone maschera bene ma a lungo andare suona cupo, come un
//! elettrodomestico. La pioggia fa lo stesso mestiere con un attacco piu'
//! naturale: tiene le alte frequenze e lascia cadere dei colpi d'acqua.

use std::time::Duration;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use rodio::Source;

use crate::suono::{CANALI, FREQUENZA};

/// Sorgente di pioggia: fruscio filtrato piu' gocce sparse.
///
/// Il rumore marrone e' caldo ma cupo, e a lungo andare suona come un
/// frigorifero. La pioggia ha la stessa funzione di mascheramento ma con
/// un attacco piu' naturale: un fruscio con le alte frequenze ancora
/// presenti e delle gocce che cadono a caso.
pub struct Pioggia {
    /// Generatore di numeri casuali.
    rng: StdRng,
    /// Memoria del filtro passa-basso: tiene il fruscio, toglie il sibilo.
    filtro: f32,
    /// Ampiezza della goccia in corso (zero quando non ce n'e').
    goccia: f32,
    /// Modulazione lenta delle raffiche.
    raffica: f32,
}

impl Pioggia {
    /// Crea un generatore di pioggia.
    pub fn nuovo() -> Self {
        Self {
            rng: StdRng::from_entropy(),
            filtro: 0.0,
            goccia: 0.0,
            raffica: 0.0,
        }
    }
}

impl Iterator for Pioggia {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let bianco: f32 = self.rng.gen::<f32>() * 2.0 - 1.0;
        // Passa-basso debole: a differenza del rumore marrone non integra,
        // quindi resta il fruscio invece del rombo.
        self.filtro += 0.45 * (bianco - self.filtro);
        let fruscio = self.filtro * 0.30;

        // Gocce: rumore breve che decade in fretta, come un colpo d'acqua.
        if self.rng.gen_bool(0.0005) {
            self.goccia = self.rng.gen::<f32>() * 0.6 + 0.3;
        }
        let goccia = if self.goccia > 0.0005 {
            let colpo = self.rng.gen::<f32>() * 2.0 - 1.0;
            self.goccia *= 0.9990;
            colpo * self.goccia * 0.35
        } else {
            self.goccia = 0.0;
            0.0
        };

        // Raffiche lente: senza, la pioggia sembra un fruscio elettronico.
        self.raffica += 0.00002 * (self.rng.gen::<f32>() * 2.0 - 1.0 - self.raffica);
        let volume = (0.85 + self.raffica * 2.5).clamp(0.6, 1.1);
        Some((fruscio + goccia) * volume)
    }
}

impl Source for Pioggia {
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

#[cfg(test)]
mod test {
    use super::*;

    /// La pioggia non deve essere muta, non deve gracchiare e non deve
    /// clippare: sono gli unici tre difetti che non si vedono a occhio.
    #[test]
    fn la_pioggia_ha_un_livello_udibile_e_pulito() {
        let mut pioggia = Pioggia::nuovo();
        let campioni: Vec<f32> = (0..FREQUENZA as usize * 2).map(|_| pioggia.next().unwrap()).collect();

        let mut somma = 0.0_f64;
        let mut picco = 0.0_f32;
        for campione in &campioni {
            assert!(campione.is_finite(), "campione non finito");
            somma += (*campione as f64) * (*campione as f64);
            picco = picco.max(campione.abs());
        }
        let rms = (somma / campioni.len() as f64).sqrt() as f32;

        // Udibile ma non invadente: il volume finale lo decide il sink.
        assert!(rms > 0.02, "troppo piano: rms={rms}");
        assert!(rms < 0.5, "troppo forte: rms={rms}");
        assert!(picco < 1.0, "clipping: picco={picco}");
        // Le gocce devono muovere il segnale, non lasciarlo piatto.
        let mut diverso = false;
        for coppia in campioni.windows(2) {
            if (coppia[0] - coppia[1]).abs() > 0.05 {
                diverso = true;
                break;
            }
        }
        assert!(diverso, "segnale piatto");
    }
}
