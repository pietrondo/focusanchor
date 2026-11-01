# FocusAnchor

App desktop nativa per Windows pensata per chi studia con ADHD e dislessia:
un solo obiettivo alla volta, blocchi brevi con premio immediato, e una voce
che legge al posto tuo.

Nessun server, nessun Electron: un singolo eseguibile Rust con `eframe`/`egui`.

## Cosa fa

**Scudo anti-distrazione (solo avvisi)**

- Nessun blocco dei siti: l'app **non modifica il file `hosts`** e non chiude
  le tue finestre. Non serve più avviarla come Amministratore.
- Ogni 800 ms controlla la finestra in primo piano: se il titolo contiene una
  delle parole che ti fanno perdere il filo, te lo dice a voce e a schermo.

**Studiare senza addormentarsi**

- Rumore marrone procedurale per coprire i rumori e i pensieri intrusivi.
- Battimento binaurale 40 Hz (200 Hz a sinistra, 240 Hz a destra) per la
  sonnolenza da sotto-stimolazione. Serve un paio di cuffie.
- **S.O.S. Sonnolenza**: 60 secondi di micro-missione fisica scelta a caso.
- **Domande a tradimento** ogni 8-14 minuti: ti chiede di riassumere un
  passaggio che hai davvero sottolineato su Zotero.

**Interleaving fra più testi**

Scegli 2 o 3 testi dello stesso esame: la sessione ruota fra loro ogni
10-12 minuti, mette in pausa il countdown e ti annuncia il testo successivo.
Alternare i testi tiene alta l'attenzione più di leggere lo stesso libro per ore.

**Gamification che non si azzera**

Punti XP, gradi (*Iniziato*, *Studioso*, *Centurione del Focus*) e micro-task
da 1, 2, 3, 5 o 10 minuti con premio immediato. Gli XP si salvano su disco:
riapri l'app e ritrovi il tuo grado. Ogni distrazione sventata costa 25 XP.

**Pausa vera**

Un pulsante **Pausa 5 minuti** ferma il timer senza penalità e riprende da
solo. È diverso dallo sblocco d'emergenza, che serve per abbandonare.

**Storico**

Ogni sessione conclusa lascia una riga in `storico.json`: quanto hai studiato,
su quale esame, quanti XP. Il riquadro *Storico* mostra il tempo per esame,
così vedi dove sei indietro.

**Scheda di ogni esame**

Per ogni esame puoi salvare la pagina del corso, i crediti, la modalità
(scritto/orale), se è da non frequentante e la data dell'appello. La data
diventa un conto alla rovescia mostrato durante lo studio.

**Ripasso dilazionato**

Ogni risposta alle domande a tradimento diventa una carta in
`anki_<esame>.txt`, in formato tabulato che Anki importa direttamente.

**Assistente AI**

Il pulsante **Spiegami questo passaggio** manda il testo copiato a OpenCode
(modello `opencode-go/deepseek-v4.1-flash`, configurabile) e mostra la
spiegazione nell'overlay. Usa l'autenticazione che hai già: nessuna chiave
nel programma e nessuna dipendenza HTTP aggiuntiva.

**Integrazione Zotero**

- Legge il database locale di Zotero con uno snapshot coerente (API di backup
  di SQLite), quindi Zotero può restare aperto.
- Menu degli esami (le collezioni Zotero) e dei testi al loro interno, con
  **lettura ricorsiva**: i materiali nelle sottocollezioni contano come parte
  dell'esame.
- Rileva il PDF che stai leggendo dalla finestra di Zotero.
- Avvisa se apri un PDF di un altro esame (cooldown di 15 secondi).
- Genera una scheda per NotebookLM divisa per esame.

**Lettura ad alta voce**

Legge il testo copiato con `Ctrl+C`, a velocità 1x, 3x o 5x. Gli a-capo
spezzati dei PDF vengono ricomposti.

## Requisiti

- Windows 11
- [Zotero](https://www.zotero.org/) 7 o successivo (per l'integrazione)
- Rust stabile (`rustup`) per compilare

## Compilare ed eseguire

```powershell
cargo build --release
.\target\release\focusanchor.exe
```

**Avvia come Amministratore** se vuoi il blocco dei domini: senza permessi
elevati resta attivo solo il Guardiano Finestre (Livello 2).

## Uso con Zotero

1. Apri Zotero e il PDF che vuoi studiare.
2. In FocusAnchor premi **Carica esami da Zotero**.
3. Scegli l'esame (la collezione) e spunta 2 o 3 testi.
4. Premi **Inizia la sessione**.

Durante la sessione:

- **Leggi il testo copiato** legge ad alta voce quello che hai copiato con `Ctrl+C`.
- **Spiega con NotebookLM** compone una domanda con l'esame giusto, la mette
  negli appunti e apre il notebook di quell'esame (se hai salvato il link).
  La incolli nella chat e premi Invio.
- **Genera scheda per NotebookLM** crea `notebooklm_<esame>.md` con le
  sottolineature e i tuoi riassunti, da caricare su NotebookLM.

L'overlay si apre nell'angolo in basso a destra, così non copre le schede e la
barra degli strumenti di Zotero. Puoi comunque trascinarlo dove preferisci.

## File creati accanto all'eseguibile

| File | Contenuto |
| --- | --- |
| `config.json` | Impostazioni e liste dei domini bloccati |
| `parcheggio_pensieri.txt` | Idee fuori tema salvate durante lo studio |
| `riassunto_studio.md` | Risposte alle domande a tradimento |
| `notebooklm_<esame>.md` | Materiale pronto per NotebookLM |

## Privacy

Tutto resta sul tuo computer. L'unica cosa che esce è ciò che decidi tu di
incollare nella chat di NotebookLM.

## Note sull'evidenza
Il rumore marrone e le tecniche di active recall hanno basi solide. Il
battimento binaurale a 40 Hz ha prove **miste**: alcuni studi mostrano un
miglioramento dell'attenzione, altri nessun effetto. È a basso rischio, ma
non è una bacchetta magica: se ti distrae, spegnilo.

## Struttura del codice

| Modulo | Responsabilità |
| --- | --- |
| `main.rs` | Avvio, panic hook, cambio finestra |
| `state.rs` | Stato dell'applicazione |
| `sessione.rs` | Timer di studio, pause, richiami |
| `blocker.rs` | File `hosts`, flush DNS, API Win32 |
| `guardia.rs` | Sorveglianza finestre e cambio contesto |
| `interleaving.rs` | Rotazione fra i testi |
| `anti_sleep.rs` | S.O.S. sonnolenza e domande a tradimento |
| `audio.rs` / `audio_voce.rs` | Sottofondo procedurale e sintesi vocale |
| `suono.rs` | Generatori audio (rumore, binaurale, beep) |
| `zotero.rs` / `zotero_query.rs` | Snapshot e query su Zotero |
| `notebooklm.rs` | Ponte verso NotebookLM |
| `task.rs` / `xp.rs` | Micro-task e punti |
| `ui*.rs` | Interfaccia |

## Test

```powershell
cargo test
```

Il test che legge il database Zotero reale è marcato `#[ignore]` perché
richiede Zotero installato:

```powershell
cargo test zotero -- --ignored --nocapture
```
