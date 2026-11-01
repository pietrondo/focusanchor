//! Punti XP e "Grado di Concentrazione".
//!
//! Il sistema e' volutamente semplice: un contatore di punti e una soglia
//! per il grado. Il feedback deve essere immediato e leggibile, non un
//! sistema complesso da capire mentre si studia.

/// Punti XP persi a ogni distrazione sventata.
pub const XP_PER_DISTRAZIONE: i32 = 25;

/// Soglia di XP per il grado "Studioso".
pub const SOGLIA_STUDIOSO: i32 = 100;

/// Soglia di XP per il grado "Centurione del Focus".
pub const SOGLIA_CENTURIONE: i32 = 300;

/// Contatore di XP con il grado corrispondente.
#[derive(Clone, Copy, Debug, Default)]
pub struct PuntiXp {
    /// Punti XP accumulati nella giornata.
    pub punti: i32,
    /// Numero di distrazioni sventate nella giornata.
    pub distrazioni: u32,
}

impl PuntiXp {
    /// Aggiunge (o sottrae) punti XP.
    pub fn aggiungi(&mut self, punti: i32) {
        self.punti += punti;
    }

    /// Registra una distrazione sventata e toglie i punti XP.
    pub fn registra_distrazione(&mut self) {
        self.distrazioni += 1;
        self.punti -= XP_PER_DISTRAZIONE;
        // I punti non scendono mai sotto zero: restare in negativo demotiverebbe.
        if self.punti < 0 {
            self.punti = 0;
        }
    }

    /// Nome del grado raggiunto in base ai punti accumulati.
    pub fn grado(&self) -> &'static str {
        if self.punti >= SOGLIA_CENTURIONE {
            "Centurione del Focus"
        } else if self.punti >= SOGLIA_STUDIOSO {
            "Studioso"
        } else {
            "Iniziato"
        }
    }

    /// Prossima soglia da raggiungere, con i punti che mancano.
    ///
    /// Restituisce `None` quando il grado massimo e' stato raggiunto.
    pub fn prossima_soglia(&self) -> Option<(i32, &'static str)> {
        if self.punti >= SOGLIA_CENTURIONE {
            None
        } else if self.punti >= SOGLIA_STUDIOSO {
            Some((SOGLIA_CENTURIONE, "Centurione del Focus"))
        } else {
            Some((SOGLIA_STUDIOSO, "Studioso"))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_grado_sale_con_i_punti() {
        let mut xp = PuntiXp::default();
        assert_eq!(xp.grado(), "Iniziato");
        xp.aggiungi(SOGLIA_STUDIOSO);
        assert_eq!(xp.grado(), "Studioso");
        xp.aggiungi(SOGLIA_CENTURIONE);
        assert_eq!(xp.grado(), "Centurione del Focus");
    }

    #[test]
    fn la_distrazione_tolga_punti_ma_non_scende_sotto_zero() {
        let mut xp = PuntiXp::default();
        xp.registra_distrazione();
        assert_eq!(xp.punti, 0);
        assert_eq!(xp.distrazioni, 1);
        xp.aggiungi(50);
        xp.registra_distrazione();
        assert_eq!(xp.punti, 25);
        assert_eq!(xp.distrazioni, 2);
    }

    #[test]
    fn la_prossima_soglia_e_nulla_al_grado_massimo() {
        let mut xp = PuntiXp {
            punti: SOGLIA_CENTURIONE,
            distrazioni: 0,
        };
        assert!(xp.prossima_soglia().is_none());
        xp.punti = 0;
        assert_eq!(xp.prossima_soglia().unwrap().1, "Studioso");
    }
}
