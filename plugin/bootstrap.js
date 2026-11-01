/* FocusAnchor dentro Zotero.
 *
 * Due mestieri, in un file solo:
 *  - dire all'app quale documento stai guardando (`focusanchor_attivo.json`);
 *  - comandarla dal pannello nella barra laterale dell'item, senza cambiare
 *    finestra: avvia, pausa, ferma, ripasso.
 *
 * Il canale e' un file nella cartella dati di Zotero, che entrambi conoscono
 * gia': niente porte, niente server, niente chiavi. Se l'app e' spenta il
 * plugin la avvia e il comando resta in attesa finche' lei non lo consuma.
 */

var FocusAnchor = {
	id: "focusanchor@locale",
	rootURI: "",
	timer: null,
	ultimo: null,
	ultimaScrittura: 0,
	_timerPannello: null,

	async startup({ id, rootURI }) {
		this.id = id;
		this.rootURI = rootURI;
		try {
			this.registraPannello();
		} catch (e) {
			Zotero.logError(new Error("FocusAnchor: pannello non registrato: " + e));
		}
		// Ogni due secondi: questo file dice "sei qui adesso", non e' un log.
		this.timer = setInterval(() => {
			this.scrivi();
		}, 2000);
		await this.scrivi();
	},

	shutdown() {
		if (this.timer) {
			clearInterval(this.timer);
			this.timer = null;
		}
		if (this._timerPannello) {
			clearInterval(this._timerPannello);
			this._timerPannello = null;
		}
	},

	install() {},
	uninstall() {},

	/** Il pannello nella barra laterale dell'item. */
	registraPannello() {
		let icona = this.rootURI + "chrome/content/icons/focusanchor.svg";
		Zotero.ItemPaneManager.registerSection({
			paneID: "focusanchor",
			pluginID: this.id,
			header: {
				l10nID: "focusanchor-header",
				icon: icona,
			},
			sidenav: {
				l10nID: "focusanchor-sidenav",
				icon: icona,
			},
			bodyXHTML: `
				<div class="focusanchor-pane" style="padding: 2px 4px 6px; font-size: 12px;">
					<div id="focusanchor-stato" style="margin-bottom: 6px; opacity: 0.85;">App chiusa</div>
					<div style="display: flex; gap: 6px; flex-wrap: wrap;">
						<button id="focusanchor-avvia" type="button">▶ Avvia</button>
						<button id="focusanchor-pausa" type="button">⏸ Pausa</button>
						<button id="focusanchor-ferma" type="button">⏹ Ferma</button>
						<button id="focusanchor-ripasso" type="button">🔁 Ripasso</button>
					</div>
				</div>`,
			onRender: (dati) => this.disegna(dati),
			onDestroy: () => {
				if (this._timerPannello) {
					clearInterval(this._timerPannello);
					this._timerPannello = null;
				}
			},
		});
	},

	/** Collega i pulsanti e tiene aggiornato lo stato. */
	disegna({ body, item }) {
		const esame = this.esameDi(item);
		const avvia = body.querySelector("#focusanchor-avvia");
		const pausa = body.querySelector("#focusanchor-pausa");
		const ferma = body.querySelector("#focusanchor-ferma");
		const ripasso = body.querySelector("#focusanchor-ripasso");
		if (!avvia) {
			return;
		}
		avvia.onclick = () => this.invia("avvia", esame);
		pausa.onclick = () => this.invia("pausa");
		ferma.onclick = () => this.invia("ferma");
		ripasso.onclick = () => this.invia("ripasso");

		this.aggiorna(body);
		if (this._timerPannello) {
			clearInterval(this._timerPannello);
		}
		this._timerPannello = setInterval(() => this.aggiorna(body), 2000);
	},

	/** Riga di stato letta dal battito dell'app. */
	async aggiorna(body) {
		const el = body.querySelector("#focusanchor-stato");
		if (!el) {
			return;
		}
		try {
			const testo = await Zotero.File.getContentsAsync(
				PathUtils.join(Zotero.DataDirectory.dir, "focusanchor_vivo.json")
			);
			const dati = JSON.parse(testo);
			// Un battito fermo significa app chiusa, non "stato sconosciuto".
			if (Date.now() - (dati.aggiornato || 0) > 6000) {
				el.textContent = "App chiusa — premi Avvia";
				return;
			}
			if (!dati.inSessione) {
				el.textContent = "Nessuna sessione";
				return;
			}
			const minuti = Math.floor((dati.secondi || 0) / 60);
			const secondi = String((dati.secondi || 0) % 60).padStart(2, "0");
			const testa = dati.inPausa ? "In pausa" : "In corso";
			const esame = dati.esame ? " · " + dati.esame : "";
			el.textContent = `${testa} · ${minuti}:${secondi}${esame}`;
		} catch (e) {
			el.textContent = "App chiusa — premi Avvia";
		}
	},

	/** Collezione di primo livello dell'item: e' l'esame per l'app. */
	esameDi(item) {
		try {
			if (!item) {
				return "";
			}
			const padre = item.parentItem || item;
			const id = padre.getCollections ? padre.getCollections()[0] : null;
			let collezione = id ? Zotero.Collections.get(id) : null;
			while (collezione && collezione.parentID) {
				collezione = Zotero.Collections.get(collezione.parentID);
			}
			return collezione ? collezione.name : "";
		} catch (e) {
			return "";
		}
	},

	/** Scrive il comando e, se serve, avvia l'app. */
	async invia(comando, esame) {
		try {
			const dati = {
				// Id unico: l'app non esegue due volte lo stesso comando.
				id: Date.now() + "-" + Math.random().toString(36).slice(2),
				comando,
				esame: esame || "",
			};
			await Zotero.File.putContentsAsync(
				PathUtils.join(Zotero.DataDirectory.dir, "focusanchor_comando.json"),
				JSON.stringify(dati)
			);
			await this.assicuraApp();
		} catch (e) {
			Zotero.debug("FocusAnchor: comando fallito: " + e);
		}
	},

	/** Avvia l'app solo se il battito dice che e' spenta. */
	async assicuraApp() {
		try {
			const testo = await Zotero.File.getContentsAsync(
				PathUtils.join(Zotero.DataDirectory.dir, "focusanchor_vivo.json")
			);
			const dati = JSON.parse(testo);
			if (Date.now() - (dati.aggiornato || 0) < 6000) {
				return;
			}
		} catch (e) {
			// Nessun battito: l'app non e' mai partita o e' chiusa.
		}

		// L'app lascia qui il proprio percorso al primo avvio: cosi' il
		// plugin non deve sapere dove sta il file.
		let eseguibile =
			"C:\\Users\\pietr\\progetti\\anti-distrazione\\target\\release\\focusanchor.exe";
		try {
			const salvato = await Zotero.File.getContentsAsync(
				PathUtils.join(Zotero.DataDirectory.dir, "focusanchor_exe.txt")
			);
			if (salvato && salvato.trim()) {
				eseguibile = salvato.trim();
			}
		} catch (e) {}

		try {
			await Zotero.Utilities.Internal.exec(eseguibile, []);
		} catch (e) {
			Zotero.debug("FocusAnchor: avvio fallito: " + e);
		}
	},

	/** L'item che stai guardando: il PDF nel lettore, o quello selezionato. */
	leggi() {
		let item = null;

		try {
			const reader =
				Zotero.Reader && Zotero.Reader.getCurrentReader
					? Zotero.Reader.getCurrentReader()
					: null;
			if (reader && reader.itemID) {
				item = Zotero.Items.get(reader.itemID);
			}
		} catch (e) {
			Zotero.debug("FocusAnchor: lettore non leggibile: " + e);
		}

		if (!item) {
			try {
				const pane = Zotero.getActiveZoteroPane();
				const scelti = pane ? pane.getSelectedItems() : [];
				if (scelti && scelti.length) {
					item = scelti[0];
				}
			} catch (e) {
				Zotero.debug("FocusAnchor: selezione non leggibile: " + e);
			}
		}

		if (!item) {
			return null;
		}

		const padre = item.parentItem || item;
		const collezioni = padre.getCollections ? padre.getCollections() : [];
		const primo = collezioni.length ? Zotero.Collections.get(collezioni[0]) : null;

		return {
			chiave: item.key,
			chiavePadre: padre.key,
			titolo: padre.getField("title") || item.getField("title") || "",
			tipo: padre.itemType || "",
			collezione: primo ? primo.name : "",
			collezioneID: primo ? primo.id : null,
			aggiornato: new Date().toISOString(),
		};
	},

	async scrivi() {
		try {
			const dati = this.leggi();
			const testo = JSON.stringify(dati);
			// Si riscrive anche quando non cambia niente, ogni 15 secondi:
			// l'app considera vecchio un file fermo, e un documento aperto
			// per un'ora smetterebbe di essere riconosciuto.
			const fermo = Date.now() - (this.ultimaScrittura || 0) > 15000;
			if (testo === this.ultimo && !fermo) {
				return;
			}
			this.ultimo = testo;
			this.ultimaScrittura = Date.now();
			// `PathUtils.join` e' obbligatorio: concatenare a mano produce
			// percorsi misti (`C:\.../file.json`) che Gecko rifiuta con
			// NS_ERROR_FILE_UNRECOGNIZED_PATH.
			const percorso = PathUtils.join(
				Zotero.DataDirectory.dir,
				"focusanchor_attivo.json"
			);
			await Zotero.File.putContentsAsync(percorso, testo);
		} catch (e) {
			Zotero.debug("FocusAnchor: scrittura fallita: " + e);
		}
	},
};

function startup(data) {
	return FocusAnchor.startup(data);
}

function shutdown() {
	return FocusAnchor.shutdown();
}

function install() {}

function uninstall() {}
