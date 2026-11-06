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
- **Pioggia naturale**: legge `pioggia.mp3` accanto all'eseguibile, così al
  posto del rumore generato ci metti la registrazione che preferisci.
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

**Tutti gli appelli in una vista**

Il riquadro *I tuoi appelli* mette in fila gli esami con la data, il conto
alla rovescia, i crediti e quanti documenti hai già letto. La data si scrive
come la vedi sul portale (`16/12/2026`) o in formato ISO (`2026-12-16`):
sono lette allo stesso modo.

Per ogni esame calcola anche il **piano di studio**: quanti minuti al giorno
ti servono per arrivare all'appello. La stima usa il tuo tempo reale di
lettura, non una media generica. Se il ritmo crolla, l'overlay te lo dice
mentre studi.

Cliccando il nome dell'esame lo selezioni: l'elenco degli appelli è anche il
modo per scegliere cosa studiare adesso.

**Ripasso con un tetto**

Le carte da ripassare non sono infinite: 15 al giorno. Davanti a un backlog
di quaranta carte si rinuncia, e le carte che saltano un giorno tornano il
giorno dopo.

**Ripasso dilazionato**

Ogni risposta alle domande a tradimento diventa una carta in
`anki_<esame>.txt`, in formato tabulato che Anki importa direttamente.

**Assistente AI**

Il pulsante **Spiegami questo passaggio** manda il testo copiato a OpenCode
(modello `opencode-go/deepseek-v4.1-flash`, configurabile) e mostra la
spiegazione nell'overlay. Usa l'autenticazione che hai già: nessuna chiave
nel programma e nessuna dipendenza HTTP aggiuntiva.

**Chiedere a NotebookLM, dentro l'app**

Il riquadro *Chiedi a NotebookLM* parla con la CLI di
[notebooklm-py](https://github.com/teng-lin/notebooklm-py): la domanda va
al notebook dell'esame e la risposta arriva nella finestra, senza passare
dal browser. Non serve Zotero aperto: gli esami sono quelli con una scheda.

- **🔗 Collega i notebook per nome** legge i tuoi notebook dall'account e li
  abbina agli esami confrontando i titoli. Un click invece di cinque
  copia-incolla di link.
- **❓ Chiedi a NotebookLM** manda la domanda scritta nel campo (anche con
  Invio) e mostra la risposta con le sue fonti.
- **📄 Fonti** elenca cosa hai già caricato in quel notebook.
- **⬆ Carica la scheda** genera la scheda dell'esame e la carica come fonte,
  senza trascinare file nel browser.
- **🎴 Genera flashcard** fa scrivere a NotebookLM un mazzo dalle fonti, e
  **Importa nel ripasso** le fa entrare nel ripasso dilazionato dell'esame.
- **💾 Salva come carta** mette una singola risposta nel ripasso, con la tua
  domanda davanti e la spiegazione dietro.

Resta anche il ripiego a mano (*copia la domanda*): la libreria non è
ufficiale e Google può cambiare le API interne. Se si rompe, il
copia-incolla funziona ancora.

**Requisiti per NotebookLM**

Una volta sola, da terminale:

```powershell
pip install notebooklm-py
notebooklm login
```

`notebooklm login` apre una finestra per l'accesso Google e salva i cookie in
`~/.notebooklm/profiles/default/`. L'app riusa quell'accesso: nessuna chiave
nel programma, nessun server. Se compare "NotebookLM non è autenticato",
rilancia `notebooklm login`.

**Integrazione Zotero**

- Legge il database locale di Zotero con uno snapshot coerente (API di backup
  di SQLite), quindi Zotero può restare aperto.
- Menu degli esami (le collezioni Zotero) e dei testi al loro interno, con
  **lettura ricorsiva**: i materiali nelle sottocollezioni contano come parte
  dell'esame.
- Ogni testo ha il pulsante **📄**: apre direttamente il PDF in Zotero
  (`zotero://open-pdf/library/items/…`). Il vecchio `↗` selezionava l'item
  nella libreria, cioè un click per poi aprirlo lo stesso.
- **🔄** rifà lo snapshot: i file aggiunti in Zotero compaiono subito, senza
  ricaricare tutto da capo.
- Rileva il PDF che stai leggendo dalla finestra di Zotero.
- Avvisa se apri un PDF di un altro esame (cooldown di 15 secondi).
- Genera una scheda per NotebookLM divisa per esame.
- Le tue sottolineature dell'esame entrano come contesto nel prompt
  dell'assistente AI: la spiegazione segue il taglio del corso, non è generica.

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

Non serve avviare come Amministratore: l'app non tocca il file `hosts` e
non chiude nessuna finestra.

## Uso con Zotero

1. Apri Zotero e il PDF che vuoi studiare.
2. In FocusAnchor premi **Carica esami da Zotero**.
3. Scegli l'esame (la collezione) e spunta 2 o 3 testi.
4. Premi **Inizia la sessione**.

**La finestra di lavoro**

All'apertura ci sono due pannelli affiancati: **Zotero a sinistra** (con il
suo pulsante *Aggancia Zotero qui*) e **NotebookLM a destra**. In mezzo c'è
una linea: trascinala per dare più spazio all'uno o all'altro. La posizione
resta salvata.

L'intestazione parte compressa (**▴** in alto a destra per allargarla):
resta una riga con XP e conto alla rovescia, e lo spazio va allo studio.
Le schede (obiettivo, testi, appelli, archivio, ripasso, storico) stanno in
*⚙ Impostazioni e appelli*, chiuso per default.

**Zotero dentro la finestra**

Il pulsante **📌 Aggancia Zotero qui** porta la finestra di Zotero dentro
FocusAnchor: i PDF si aprono e si sottolineano senza cambiare finestra. Se
Zotero non è aperto, l'app lo avvia e aspetta che la finestra esista.

- **⏏ Stacca Zotero** lo rimette come finestra indipendente.
- All'uscita dall'app Zotero viene sempre staccato: una finestra agganciata
  a una finestra chiusa sparirebbe dallo schermo.
- Durante una sessione viene staccato da solo: l'overlay è 420×380 e Zotero
  non ci sta.

È un aggancio di finestre (`SetParent`), non un'API pensata per questo:
Microsoft non lo documenta come supportato fra processi diversi. Funziona,
ma se Zotero si comporta in modo strano il rimedio è **Stacca**.

Durante la sessione:

- **Leggi il testo copiato** legge ad alta voce quello che hai copiato con `Ctrl+C`.
- **Spiega con NotebookLM** compone una domanda con l'esame giusto, la mette
  negli appunti e apre il notebook di quell'esame (se hai salvato il link).
  La incolli nella chat e premi Invio.
- **Genera scheda per NotebookLM** crea `notebooklm_<esame>.md` con le
  sottolineature e i tuoi riassunti, da caricare su NotebookLM. **📂 Mostra il
  file** lo apre già selezionato in Esplora file, così lo trascini nel
  notebook senza cercarlo.
L'overlay si apre nell'angolo in basso a destra, così non copre le schede e la
barra degli strumenti di Zotero. Puoi comunque trascinarlo dove preferisci.

## Il ciclo di studio, in un posto solo

L'app non è un timer con sei comandi: è un ciclo che si ripete per ogni esame.

1. **Materiale** — gli esami sono le collezioni Zotero, i testi sono i
   documenti. Il tempo che passi su ciascuno viene misurato dalla finestra in
   primo piano e finisce nello storico.
2. **Studio** — la sessione: timer, pausa vera, guardia anti-distrazione,
   lettura ad alta voce.
3. **Verifica** — le domande a tradimento chiedono di spiegare un passaggio del
   testo che stai leggendo. La tua risposta è il materiale della fase dopo.
4. **Ripasso** — ogni risposta diventa una carta in `esame_<nome>.json` e torna
   dentro l'app nei giorni giusti (ripetizione dilazionata). Non serve aprire
   Anki: il giro studio → dubbio → ripasso resta in una finestra sola.
5. **Stato** — risposte date, ore per documento, testi contro riassunti, giorni
   all'appello: tutto nel riquadro *Storico*.

Il file per Anki resta e si può esportare (`anki_<esame>.txt`), ma è un'uscita,
non il posto dove ripassi.

## Estensione Zotero (opzionale)

L'app riconosce il documento che stai leggendo dal titolo della finestra, e
sbaglia: la finestra tronca i titoli lunghi e un PDF aperto in una seconda
finestra non si vede. `plugin/` contiene una piccola estensione che scrive la
**chiave esatta** dell'item aperto in `<cartella Zotero>/focusanchor_attivo.json`.

Installazione (una volta):

```powershell
.\plugin\build.ps1                 # crea plugin/focusanchor.xpi
```

Poi in Zotero: **Strumenti → Componenti aggiuntivi → ingranaggio → Install
Add-on From File…** e scegli `plugin/focusanchor.xpi`.

Va installata a mano: su Zotero 10 il caricamento automatico di un `.xpi`
copiato nel profilo non funziona (l'ho provato: il gestore degli addon ignora
il file finché non passa dall'interfaccia). Senza estensione l'app funziona
lo stesso, solo meno precisa.

## Comandare tutto da Zotero

Con l'estensione installata, FocusAnchor si comanda **dal pannello nella barra
laterale dell'item** (a destra del PDF, sotto le note): ▶ Avvia, ⏸ Pausa,
⏹ Ferma, 🔁 Ripasso.

- **Avvia** parte sull'esame a cui appartiene il documento aperto (la
  collezione di primo livello), quindi non devi scegliere niente.
- Se l'app è spenta il plugin **la avvia da solo** e le lascia il comando in
  attesa: non devi aprire nient'altro.
- La riga di stato dice cosa sta succedendo (`In corso · 12:34 · storia
  romana`), letta dal battito che l'app scrive ogni due secondi.

Il canale è un file nella cartella dati di Zotero (`focusanchor_comando.json`):
niente porte aperte, niente server, niente chiavi.

## I file che crea l'app

I dati stanno in `%LOCALAPPDATA%\FocusAnchor`: li' si puo' sempre scrivere,
e non spariscono quando il programma viene spostato o reinstallato. La
prima volta, i file delle versioni vecchie (accanto all'eseguibile) vengono
portati li' automaticamente.

| File | Contenuto |
| --- | --- |
| `config.json` | Impostazioni, XP e parole chiave |
| `pioggia.mp3` | Pioggia registrata della modalità *Pioggia naturale* (sostituibile) |
| `parcheggio_pensieri.txt` | Idee fuori tema salvate durante lo studio |
| `riassunto_studio.md` | Risposte alle domande a tradimento (leggibile a occhio) |
| `esame_<esame>.json` | Carte di ripasso di quell'esame, con le scadenze |
| `anki_<esame>.txt` | Le stesse carte, in formato tabulato per Anki |
| `notebooklm_<esame>.md` | Materiale pronto per NotebookLM |
| `storico.json` | Sessioni concluse, XP e tempo per documento |

Il file `pioggia.mp3` si cerca prima accanto all'eseguibile e poi nella
cartella dei dati.

## Backup dei dati

I tuoi dati sono in `%LOCALAPPDATA%\FocusAnchor`: se reinstalli Windows,
spariscono. Il bottone **💾 Esporta** (nel riquadro *I tuoi file*) ne fa
una copia in una cartella sulla scrivania o nei documenti, e te la apre
nella cartella: da li' la sposti dove vuoi, magari in una cartella già
sincronizzata con il cloud.

Per ripristinare basta il contrario: copia i file dell'esportazione
indietro in `%LOCALAPPDATA%\FocusAnchor` e riavvia l'app. Il backup
 contiene gli stessi file che l'app usa (`config.json`, `storico.json`,
`esame_*.json`), quindi niente va riconvertito.

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
| `main.rs` | Avvio, migrazione dei dati, cambio finestra |
| `state.rs` | Stato dell'applicazione |
| `sessione.rs` | Timer di studio, pause, richiami |
| `blocker.rs` | Pulizia del file `hosts` e API Win32 |
| `guardia.rs` | Sorveglianza finestre e cambio contesto |
| `interleaving.rs` | Rotazione fra i testi |
| `anti_sleep.rs` | S.O.S. sonnolenza e domande a tradimento |
| `audio.rs` / `audio_voce.rs` | Sottofondo procedurale e sintesi vocale |
| `suono.rs` / `suono_file.rs` / `suono_pioggia.rs` | Generatori audio e file |
| `zotero.rs` / `zotero_query.rs` | Snapshot e query su Zotero |
| `zotero_attivo.rs` | Legge cosa dice l'estensione Zotero |
| `incasso.rs` | Aggancia la finestra di Zotero dentro l'app |
| `tempo.rs` | Tempo per documento dalla finestra in primo piano |
| `carte.rs` / `carte_sottolineature.rs` | Carte di ripasso e ripetizione dilazionata |
| `storico.rs` | Sessioni concluse e statistiche |
| `ai.rs` | Assistente via `opencode run` |
| `notebooklm.rs` | Ponte verso NotebookLM |
| `task.rs` / `xp.rs` | Micro-task e punti |
| `appelli.rs` | Date degli appelli e piano di studio |
| `archivi.rs` | File dell'utente (pensieri, riassunti, schede) |
| `comando.rs` / `comandi_zotero.rs` | Canale con l'estensione Zotero |
| `config.rs` | Configurazione persistente e cartella dei dati |
| `webview.rs` | Browser incorporato per NotebookLM |
| `ui*.rs` | Interfaccia |
| `plugin/` | Estensione Zotero (manifest, bootstrap, build) |

## Test

```powershell
cargo test
```

Il test che legge il database Zotero reale è marcato `#[ignore]` perché
richiede Zotero installato:

```powershell
cargo test zotero -- --ignored --nocapture
```
