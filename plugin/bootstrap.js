/* FocusAnchor: dice all'app quale documento di Zotero e' aperto.
 *
 * L'app indovina dove sei dal titolo della finestra, e sbaglia: la finestra
 * tronca i titoli lunghi, un PDF aperto in una seconda finestra non si vede,
 * e "docsity-riassunto-…" e "Docsity riassunto…" diventano due documenti.
 * Qui invece si sa per certo quale item e' davanti, con la sua chiave.
 *
 * Il file scritto e' `focusanchor_attivo.json` nella cartella dati di Zotero,
 * che l'app trova gia' perche' la usa per leggere il database.
 */

var FocusAnchor = {
	id: "focusanchor@locale",
	timer: null,
	ultimo: null,

	async startup({ id }) {
		this.id = id;
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
	},

	install() {},
	uninstall() {},

	/** L'item che stai guardando: il PDF nel lettore, o quello selezionato. */
	leggi() {
		let item = null;

		// Caso normale mentre studi: il lettore PDF aperto.
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

		// Altrimenti vale la selezione nella libreria.
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

		// Sull'allegato PDF il titolo utile e' quello del libro che lo contiene.
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
			// Niente scritture inutili: il file cambia solo quando cambia il fuoco.
			if (testo === this.ultimo) {
				return;
			}
			this.ultimo = testo;
			const percorso = Zotero.DataDirectory.dir + "/focusanchor_attivo.json";
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
