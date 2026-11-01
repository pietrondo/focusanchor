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
	_inPausa: false,
	_esame: "",

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
				<div class="focusanchor-pane">
					<div class="focusanchor-titolo">FocusAnchor</div>
					<div class="focusanchor-riga">
						<span id="focusanchor-punto"></span>
						<span id="focusanchor-stato">App chiusa</span>
					</div>
					<div id="focusanchor-dettaglio"></div>
					<div class="focusanchor-griglia">
						<button id="focusanchor-avvia" type="button">Avvia</button>
						<button id="focusanchor-pausa" type="button">Pausa</button>
						<button id="focusanchor-ferma" type="button">Ferma</button>
						<button id="focusanchor-ripasso" type="button">Ripasso</button>
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

	/** Collega i pulsanti, dà loro una forma e tiene aggiornato lo stato. */
	disegna({ body, item }) {
		this._body = body;
		this._esame = this.esameDi(item);

		const cerca = (id) => body.querySelector("#" + id);
		const avvia = cerca("focusanchor-avvia");
		const pausa = cerca("focusanchor-pausa");
		const ferma = cerca("focusanchor-ferma");
		const ripasso = cerca("focusanchor-ripasso");
		if (!avvia) {
			return;
		}

		// Lo stile si mette qui e non in CSS: il pannello è dentro il documento
		// di Zotero e deve adattarsi al tema chiaro o scuro senza file esterni.
		// Il contenitore della sezione dispone i figli in riga: senza questi
		// vincoli il testo finisce schiacciato in una colonna da ottanta pixel.
		body.style.display = "block";
		body.style.width = "100%";
		body.style.boxSizing = "border-box";
		body.style.padding = "0";

		const pane = body.querySelector(".focusanchor-pane");
		if (pane) {
			pane.style.display = "flex";
			pane.style.flexDirection = "column";
			pane.style.width = "100%";
			pane.style.boxSizing = "border-box";
			pane.style.gap = "6px";
			pane.style.fontSize = "12px";
			pane.style.padding = "2px 0 4px";
		}

		const titolo = body.querySelector(".focusanchor-titolo");
		if (titolo) {
			titolo.style.fontSize = "10px";
			titolo.style.letterSpacing = "0.08em";
			titolo.style.textTransform = "uppercase";
			titolo.style.opacity = "0.55";
			titolo.style.fontWeight = "600";
		}

		const riga = body.querySelector(".focusanchor-riga");
		if (riga) {
			riga.style.display = "flex";
			riga.style.alignItems = "center";
			riga.style.gap = "8px";
			riga.style.width = "100%";
			riga.style.fontSize = "13px";
			riga.style.lineHeight = "1.35";
		}
		const stato = cerca("focusanchor-stato");
		if (stato) {
			stato.style.flex = "1 1 auto";
			stato.style.overflowWrap = "anywhere";
		}
		const punto = cerca("focusanchor-punto");
		if (punto) {
			punto.style.width = "8px";
			punto.style.height = "8px";
			punto.style.borderRadius = "50%";
			punto.style.background = "#9aa0a6";
			punto.style.flex = "0 0 auto";
			punto.style.display = "inline-block";
		}
		const dettaglio = cerca("focusanchor-dettaglio");
		if (dettaglio) {
			dettaglio.style.width = "100%";
			dettaglio.style.fontSize = "11px";
			dettaglio.style.opacity = "0.65";
			dettaglio.style.lineHeight = "1.35";
			dettaglio.style.overflowWrap = "anywhere";
		}
		const griglia = body.querySelector(".focusanchor-griglia");
		if (griglia) {
			griglia.style.display = "grid";
			griglia.style.gridTemplateColumns = "1fr 1fr";
			griglia.style.gap = "6px";
			griglia.style.width = "100%";
			griglia.style.marginTop = "2px";
		}
		for (const [elemento, primario] of [
			[avvia, true],
			[pausa, false],
			[ferma, false],
			[ripasso, false],
		]) {
			elemento.style.width = "100%";
			elemento.style.minHeight = "26px";
			elemento.style.padding = "5px 6px";
			elemento.style.fontSize = "12px";
			elemento.style.textAlign = "center";
			elemento.style.borderRadius = "4px";
			elemento.style.border = "1px solid rgba(128,128,128,0.45)";
			elemento.style.background = primario ? "rgba(46,204,113,0.10)" : "transparent";
			elemento.style.color = "inherit";
			elemento.style.fontWeight = primario ? "600" : "400";
			if (primario) {
				elemento.style.borderColor = "rgba(46,204,113,0.45)";
			}
		}

		avvia.onclick = () => this.invia("avvia", this._esame);
		pausa.onclick = () => this.invia(this._inPausa ? "riprendi" : "pausa");
		ferma.onclick = () => this.invia("ferma");
		ripasso.onclick = () => this.invia("ripasso");

		this.aggiorna(body);
		if (this._timerPannello) {
			clearInterval(this._timerPannello);
		}
		this._timerPannello = setInterval(() => this.aggiorna(body), 2000);
	},

	/** Riga di stato e stato dei pulsanti, letti dal battito dell'app. */
	async aggiorna(body) {
		const cerca = (id) => body.querySelector("#" + id);
		const stato = cerca("focusanchor-stato");
		const dettaglio = cerca("focusanchor-dettaglio");
		const punto = cerca("focusanchor-punto");
		const avvia = cerca("focusanchor-avvia");
		const pausa = cerca("focusanchor-pausa");
		const ferma = cerca("focusanchor-ferma");
		const ripasso = cerca("focusanchor-ripasso");
		if (!stato) {
			return;
		}

		const abilita = (elemento, attivo) => {
			if (!elemento) {
				return;
			}
			elemento.disabled = !attivo;
			elemento.style.opacity = attivo ? "1" : "0.4";
			elemento.style.cursor = attivo ? "pointer" : "default";
		};

		let dati = null;
		try {
			const testo = await Zotero.File.getContentsAsync(
				PathUtils.join(Zotero.DataDirectory.dir, "focusanchor_vivo.json")
			);
			dati = JSON.parse(testo);
			// Un battito fermo significa app chiusa, non "stato sconosciuto".
			if (Date.now() - (dati.aggiornato || 0) > 6000) {
				dati = null;
			}
		} catch (e) {
			dati = null;
		}

		if (!dati) {
			this._inPausa = false;
			stato.textContent = "App chiusa";
			dettaglio.textContent = "Premi Avvia: la apro io.";
			if (punto) {
				punto.style.background = "#9aa0a6";
			}
			if (pausa) {
				pausa.textContent = "Pausa";
			}
			abilita(avvia, true);
			abilita(pausa, false);
			abilita(ferma, false);
			abilita(ripasso, true);
			return;
		}

		this._inPausa = !!dati.inPausa;
		const carte = dati.daRipassare || 0;
		if (ripasso) {
			ripasso.textContent = carte > 0 ? `Ripasso (${carte})` : "Ripasso";
		}
		if (pausa) {
			pausa.textContent = this._inPausa ? "Riprendi" : "Pausa";
		}

		if (!dati.inSessione) {
			stato.textContent = "Nessuna sessione";
			if (punto) {
				punto.style.background = "#9aa0a6";
			}
			if (!dati.esamiCaricati) {
				dettaglio.textContent =
					"Apri FocusAnchor e premi «Carica esami da Zotero» una volta.";
			} else {
				dettaglio.textContent = this._esame
					? "Pronto su " + this._esame
					: "Scegli un documento di un esame";
			}
			abilita(avvia, true);
			abilita(pausa, false);
			abilita(ferma, false);
			return;
		}

		const minuti = Math.floor((dati.secondi || 0) / 60);
		const secondi = String((dati.secondi || 0) % 60).padStart(2, "0");
		stato.textContent = `${this._inPausa ? "In pausa" : "In corso"} · ${minuti}:${secondi}`;
		if (punto) {
			punto.style.background = this._inPausa ? "#f0a30a" : "#2ecc71";
		}
		const parti = [];
		if (dati.esame) {
			parti.push(dati.esame);
		}
		if (carte > 0) {
			parti.push(`${carte} carte da ripassare`);
		}
		dettaglio.textContent = parti.join(" · ");
		abilita(avvia, false);
		abilita(pausa, true);
		abilita(ferma, true);
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
