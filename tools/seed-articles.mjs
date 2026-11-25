// Fills a running WHERE instance with articles from event technology.
//
// Usage: node seed-articles.mjs [--no-devices] [--api http://127.0.0.1:8090]
//                               [--user admin2 --password start [--mandant 3] | --token <token>]
//
// The script only uses the public API — no direct database access. It is
// repeatable: if an article with the same name already exists it is skipped
// (no duplicate, no changed number).
//
// It sets the fields the article master maintains today: category flags, stock
// handling, manufacturer data, prices of the tabs "Miete" and "Verkauf", dimensions,
// operating values, inspection data and the six text blocks. Not settable today:
// suppliers, the search index, the `pyro_*` columns and article bundles — those
// screens and endpoints do not exist yet (see README, "Not implemented").

import { sessionHeader } from './session.mjs';

const args = { api: process.env.WHERE_API ?? 'http://127.0.0.1:8090', devices: true };
for (let i = 2; i < process.argv.length; i++) {
	if (process.argv[i] === '--api') args.api = process.argv[++i].replace(/\/+$/, '');
	else if (process.argv[i] === '--no-devices') args.devices = false;
}
args.api = args.api.replace(/\/+$/, '');

const session = await sessionHeader(args.api);

async function api(path, init) {
	const response = await fetch(`${args.api}${path}`, {
		...init,
		headers: { 'content-type': 'application/json', ...session, ...(init?.headers ?? {}) }
	});
	if (!response.ok) {
		const problem = await response.text();
		throw new Error(`${init?.method ?? 'GET'} ${path} → ${response.status}: ${problem}`);
	}
	return response.status === 204 ? null : response.json();
}

/** EAN-13 with a correct check digit; prefix 20 = internal use, no manufacturer code. */
function ean13(base) {
	const digits = String(base).padStart(12, '0').slice(0, 12).split('').map(Number);
	const sum = digits.reduce((s, d, i) => s + d * (i % 2 === 0 ? 1 : 3), 0);
	const check = (10 - (sum % 10)) % 10;
	return digits.join('') + check;
}

// ---------------------------------------------------------------------------
// Article groups
// ---------------------------------------------------------------------------
const GROUPS = ['Licht', 'Ton', 'Bühne', 'Video', 'Verbrauchsmaterial', 'Effekte'];
/** Untergruppe, damit die linke Menüspalte der Artikelliste einen Baum zeigt. */
const SUBGROUPS = [{ name: 'LED-Technik', parent: 'Licht' }];
const existingGroups = await api('/api/v1/master-data/article-groups');
const group = {};
for (const name of GROUPS) {
	const match = existingGroups.find((g) => g.name === name);
	group[name] =
		match?.id ??
		(
			await api('/api/v1/master-data/article-groups', {
				method: 'POST',
				body: JSON.stringify({ name, parent_id: null })
			})
		).id;
}
for (const sub of SUBGROUPS) {
	const match = existingGroups.find((g) => g.name === sub.name);
	group[sub.name] =
		match?.id ??
		(
			await api('/api/v1/master-data/article-groups', {
				method: 'POST',
				body: JSON.stringify({ name: sub.name, parent_id: group[sub.parent] })
			})
		).id;
}

/** Warengruppen eines Artikels: die Gruppe aus der Liste, LED-Artikel zusätzlich in der Untergruppe. */
function groupsOf(a) {
	const ids = [group[a.group]];
	if (a.group === 'Licht' && /\bLED\b/.test(a.name)) ids.push(group['LED-Technik']);
	return [...new Set(ids)].sort((x, y) => x - y);
}

// ---------------------------------------------------------------------------
// Source data
// ---------------------------------------------------------------------------
const template = await api('/api/v1/articles/new');
const existing = new Map(
	(await api('/api/v1/articles?limit=500')).items.map((a) => [a.name.trim().toLowerCase(), a.id])
);

/**
 * `group` = article group, `individual` = capture single devices (serial numbers),
 * `devices` = number of devices for the goods receipt.
 */
const ARTICLES = [
	// --------------------------------------------------------------- Lighting
	{ supplierNos: [2001, 2003], group: 'Licht', name: 'Moving Head Spot 350 W', manufacturer: 'Robe', manufacturer_no: 'ROBIN-LEDBEAM-150', retail_price: 4290.0, weight: 11.4, width: 33, height: 47, depth: 25, power: 350, voltage: [100, 240], current: 3.2, inspection_class: 2, inspection_months: 6, individual: true, devices: 4, for_sale: true, short: 'Moving Head Spot, 350 W LED, 17 Gobos, 14 Farben', long: 'Spot-Moving-Head für Bühnen- und TV-Einsatz. 350 W LED-Engine, motorischer Zoom 5–45°, Fokus und Prisma. Ansteuerung über DMX512 (3-polig XLR), Bedienung über Display oder Konsole.', technical: 'Leistung 350 W, Spannung 100–240 V, DMX 22 Kanäle, Zoom 5–45°, Gewicht 11,4 kg', scope: 'Gerät, Netzleitung, 2 Omega-Halter, Bedienungsanleitung' },
	{ group: 'Licht', name: 'Moving Head Wash 19×40 W LED', manufacturer: 'Clay Paky', manufacturer_no: 'AXCOR-WASH-300', retail_price: 5150.0, weight: 16.8, width: 45, height: 52, depth: 30, power: 760, voltage: [100, 240], current: 6.5, inspection_class: 2, inspection_months: 6, individual: true, devices: 3, for_sale: true, short: 'Wash 19×40 W RGBW, motorischer Zoom', long: 'LED-Wash mit 19 RGBW-Multichip-LEDs für gleichmäßige Flächenausleuchtung. Motorischer Zoom 6–50°, elektronischer Dimmer, RDM-fähig.', technical: 'Leistung 760 W, Zoom 6–50°, DMX 15 Kanäle, Gewicht 16,8 kg' },
	{ supplierNos: [2001], group: 'Licht', name: 'LED-PAR 64 RGBWA+UV', manufacturer: 'Eurolite', manufacturer_no: 'LED-PAR-64-RGBWAUV', retail_price: 429.0, weight: 3.1, width: 26, height: 29, depth: 24, power: 90, voltage: [100, 240], current: 0.8, inspection_class: 2, inspection_months: 12, individual: true, devices: 8, for_sale: true, short: 'LED-PAR mit 18×10 W, RGBWA+UV', long: 'Universeller LED-PAR für Bühne, Front- und Effektlicht. Farbmischung über RGBWA+UV, Standalone-Betrieb mit eingebauten Programmen oder DMX.', technical: '18×10 W LED, 90 W, DMX 6/10 Kanäle, Abstrahlwinkel 25°, Gewicht 3,1 kg' },
	{ group: 'Licht', name: 'LED-Bar 12×10 W RGBA', manufacturer: 'Showtec', manufacturer_no: 'LED-BAR-12-RGBA', retail_price: 389.0, weight: 3.4, width: 100, height: 12, depth: 10, power: 120, voltage: [100, 240], current: 1.1, inspection_class: 2, inspection_months: 12, individual: true, devices: 6, short: 'LED-Bar, 12 Segmente einzeln ansteuerbar', long: 'Lineare LED-Leiste für Wandfluter und Bodeneffekte. Segmente einzeln ansteuerbar, integrierte Programme, DMX und Standalone.', technical: '12×10 W RGBA, Schwenkbereich 220°, DMX 5/8/36 Kanäle, Gewicht 3,4 kg' },
	{ group: 'Licht', name: 'Profilscheinwerfer 750 W', manufacturer: 'ETC', manufacturer_no: 'SOURCE-FOUR-750', retail_price: 1190.0, weight: 8.2, width: 20, height: 30, depth: 65, power: 750, voltage: [230, 230], current: 3.3, inspection_class: 2, inspection_months: 12, individual: true, devices: 12, short: 'Profilscheinwerfer 750 W, HPL, Zoom 19–50°', long: 'Klassischer Profilscheinwerfer mit vier Blendflügeln und Irisaufnahme. Ideal für Frontlicht und Akzentuierung aus dem Zuschauerraum.', technical: 'HPL 750 W, Zoom 19–50°, CEE 7/7, Gewicht 8,2 kg' },
	{ group: 'Licht', name: 'Fresnel-Scheinwerfer 2 kW', manufacturer: 'ARRI', manufacturer_no: 'TRUE-BLUE-T5', retail_price: 2150.0, weight: 12.5, width: 38, height: 42, depth: 32, power: 2000, voltage: [230, 230], current: 8.7, inspection_class: 1, inspection_months: 12, individual: true, devices: 2, short: 'Fresnel 2 kW für Flächenlicht', long: 'Stufenlinsenscheinwerfer mit weichem Lichtkegel für Flächenausleuchtung. Spot-Flood-Verstellung, Bajonett für Torblenden.', technical: 'CP/41 2 kW, Spannung 230 V, Gewicht 12,5 kg, Schutzklasse I' },
	{ group: 'Licht', name: 'Verfolgerscheinwerfer 1200 W', manufacturer: 'Robert Juliat', manufacturer_no: 'LUCY-1200', retail_price: 8900.0, weight: 24.0, width: 45, height: 55, depth: 70, power: 1200, voltage: [230, 230], current: 5.5, inspection_class: 1, inspection_months: 6, individual: true, devices: 1, for_sale: false, short: 'Verfolger 1200 W mit Iris und Farbwechsler', long: 'Manueller Verfolgerscheinwerfer für Premieren und Solospots. Sechs-Farben-Wechsler, Iris, Fokus, Bedienung über Handgriffe.', technical: 'MSR 1200 W, Zoom 7–17°, Gewicht 24 kg' },
	{ group: 'Licht', name: 'Dimmerpack 6×16 A', manufacturer: 'Zero 88', manufacturer_no: 'CHILLI-DIM6', retail_price: 780.0, weight: 5.6, width: 48, height: 9, depth: 30, power: 6000, voltage: [230, 230], current: 26, inspection_class: 1, inspection_months: 12, individual: true, devices: 4, short: '6-Kanal-Dimmer, 16 A je Kanal', long: 'Digitale Dimmereinheit für konventionelles Licht. Sechs Kanäle mit je 16 A, DMX-Eingang, Absicherung über Leistungsschutzschalter.', technical: '6×16 A, 6 kW gesamt, DMX512, CEE 32 A Anschluss' },
	{ group: 'Licht', name: 'Lichtpult 48 Kanäle', manufacturer: 'MA Lighting', manufacturer_no: 'GRANDMA3-LIGHT', retail_price: 14900.0, weight: 18.0, width: 70, height: 12, depth: 50, power: 150, voltage: [100, 240], current: 1.5, inspection_class: 2, inspection_months: 12, individual: true, devices: 1, for_sale: false, short: 'Lichtpult mit 4096 Parametern', long: 'Kompakte Lichtsteuerung mit Touchscreen, zwei DMX-Universen und Netzwerkanschluss für Art-Net und sACN. Speicherung von Cues und Chasern.', technical: '4096 Parameter, 2× DMX512, Art-Net, sACN, Gewicht 18 kg' },
	{ group: 'Licht', name: 'DMX-Splitter 1→6', manufacturer: 'Elation', manufacturer_no: 'DMX-BRANCH-6', retail_price: 249.0, weight: 1.2, width: 22, height: 5, depth: 15, power: 10, voltage: [100, 240], current: 0.1, inspection_class: 2, inspection_months: 12, devices: 3, short: 'DMX-Verteiler, ein Eingang auf sechs Ausgänge', long: 'Aktiver DMX-Splitter mit sechs optoisolierten Ausgängen. Verhindert Störungen durch Masseschleifen auf langen Strecken.', technical: 'DMX512-A, 6 Ausgänge, 3-polig XLR, 10 W' },
	{ group: 'Licht', name: 'LED-Strahler RGBW 100 W', manufacturer: 'Futurelight', manufacturer_no: 'PIX-100-RGBW', retail_price: 320.0, weight: 2.4, width: 18, height: 18, depth: 16, power: 100, voltage: [100, 240], current: 0.9, inspection_class: 2, inspection_months: 12, individual: true, devices: 10, for_sale: true, short: 'Kompakter LED-Strahler mit Zoom', long: 'Kleiner LED-Scheinwerfer für Akzente, Gassen und Hintergrund. Zoom manuell, Farbmischung RGBW, DMX und Standalone.', technical: '100 W, Zoom 10–35°, DMX 3/5/8 Kanäle, IP20' },
	{ group: 'Licht', name: 'LED-Vorhanglicht 4×25 W', manufacturer: 'Eurolite', manufacturer_no: 'LED-CYC-4', retail_price: 690.0, weight: 6.8, width: 62, height: 24, depth: 18, power: 100, voltage: [100, 240], current: 0.9, inspection_class: 2, inspection_months: 12, individual: true, devices: 2, short: 'Horizontlicht für Vorhänge und Zykloramen', long: 'Asymmetrischer Strahler für die Ausleuchtung von Vorhängen und Zykloramen, vier einzeln ansteuerbare Segmente.', technical: '4×25 W RGBW, DMX 3/4/6/10 Kanäle, Gewicht 6,8 kg' },

	// ------------------------------------------------------------------ Sound
	{ group: 'Ton', name: 'Digitalmischpult 32 Kanäle', manufacturer: 'Yamaha', manufacturer_no: 'QL5', retail_price: 12500.0, weight: 22.0, width: 83, height: 27, depth: 55, power: 250, voltage: [100, 240], current: 2.5, inspection_class: 2, inspection_months: 12, individual: true, devices: 1, short: 'Digitalpult, 32 Fader, 64 Eingänge', long: 'Digitales Mischpult für Theater- und Konzertbetrieb. 64 Mono-Eingänge, 32 Fader, Szenenspeicher, Dante-Karte optional.', technical: '64 Eingangskanäle, 32 Fader, 24 Mix-Busse, Dante optional, Gewicht 22 kg' },
	{ group: 'Ton', name: 'Analogmischpult 16 Kanäle', manufacturer: 'Allen & Heath', manufacturer_no: 'ZED-16FX', retail_price: 1290.0, weight: 8.4, width: 62, height: 12, depth: 42, power: 60, voltage: [100, 240], current: 0.6, inspection_class: 2, inspection_months: 12, individual: true, devices: 2, short: 'Analogpult mit Effektgerät, 16 Kanäle', long: 'Kompaktes Analogpult für kleinere Veranstaltungen und Proben. 16 Kanäle, 4 Aux-Wege, integriertes Effektgerät.', technical: '16 Kanäle, 4 Aux, 100-mm-Fader, USB-Audiointerface' },
	{ group: 'Ton', name: 'Aktivlautsprecher 12"/1,4"', manufacturer: 'd&b audiotechnik', manufacturer_no: 'MAX12', retail_price: 3980.0, weight: 24.5, width: 40, height: 60, depth: 36, power: 800, voltage: [100, 240], current: 3.0, inspection_class: 2, inspection_months: 12, individual: true, devices: 6, for_sale: true, short: 'Aktiv-Bühnenmonitor und Front-PA', long: 'Koaxialer Aktivlautsprecher für Monitoring und kleine Frontbeschallung. Flug- und Stativbetrieb, DSP-Presets für Sprache und Musik.', technical: '12"/1,4", 800 W, 133 dB max, XLR-Eingang, Gewicht 24,5 kg' },
	{ group: 'Ton', name: 'Subwoofer 18" aktiv', manufacturer: 'RCF', manufacturer_no: 'SUB-8004-AS', retail_price: 2150.0, weight: 42.0, width: 50, height: 60, depth: 60, power: 1400, voltage: [230, 230], current: 6.0, inspection_class: 2, inspection_months: 12, individual: true, devices: 4, for_sale: true, short: 'Aktivsubwoofer für Beschallung und Bühne', long: 'Direktstrahlender 18"-Subwoofer mit Digitalendstufe und DSP. Mehrere Gehäuse koppelbar.', technical: '18", 1400 W, 133 dB, Frequenzgang 30–120 Hz, Gewicht 42 kg' },
	{ group: 'Ton', name: 'Line-Array-Element', manufacturer: 'L-Acoustics', manufacturer_no: 'KARA-II', retail_price: 6400.0, weight: 26.0, width: 74, height: 21, depth: 42, power: 1000, voltage: [100, 240], current: 4.0, inspection_class: 2, inspection_months: 12, individual: true, devices: 8, short: 'Line-Array-Element für geflogene Beschallung', long: 'Element für hängende Line-Arrays, passiv getrennt, mit Rigging-Rahmen fliegbar. Nur mit zugehörigem Controller betreiben.', technical: '2×8" + 4×3", 16 Ω, 143 dB max, Gewicht 26 kg' },
	{ group: 'Ton', name: 'Endstufe 4×1300 W', manufacturer: 'Powersoft', manufacturer_no: 'X4', retail_price: 3480.0, weight: 8.0, width: 48, height: 4, depth: 38, power: 5200, voltage: [100, 240], current: 12, inspection_class: 1, inspection_months: 12, individual: true, devices: 3, short: 'Vierkanal-DSP-Endstufe, 1 HE', long: 'Digitale Endstufe mit integriertem DSP für Monitoring und PA. Vier Kanäle, Presets, Fernsteuerung über Netzwerk.', technical: '4×1300 W an 4 Ω, DSP, 1 HE, 8 kg' },
	{ group: 'Ton', name: 'Funkmikrofon-Set UHF', manufacturer: 'Shure', manufacturer_no: 'QLXD24-SM58', retail_price: 1790.0, weight: 2.1, width: 40, height: 6, depth: 22, power: 20, voltage: [100, 240], current: 0.2, inspection_class: 2, inspection_months: 12, individual: true, devices: 6, for_sale: true, short: 'Digitales Handsender-Set mit SM58-Kapsel', long: 'Digitales Funkstrecken-Set für Sprache und Gesang. AES-256-Verschlüsselung, 64 MHz Schaltbandbreite, Akku- oder Batteriebetrieb.', technical: 'UHF 470–616 MHz, 24 Bit, Reichweite bis 100 m, 2,1 kg' },
	{ group: 'Ton', name: 'DI-Box aktiv', manufacturer: 'Radial', manufacturer_no: 'J48', retail_price: 219.0, weight: 0.5, width: 8, height: 5, depth: 12, power: null, voltage: [48, 48], current: null, inspection_class: null, inspection_months: null, individual: true, devices: 6, for_sale: true, short: 'Aktive DI-Box, phantomgespeist', long: 'Aktive DI-Box zur symetrischen Übertragung von Instrumentensignalen, Phantomspeisung oder Batterie.', technical: '48 V Phantomspeisung, 10 MΩ Eingang, XLR-Ausgang, 0,5 kg' },

	// ------------------------------------------------------------------ Stage
	{ group: 'Bühne', name: 'Traverse 3-Punkt 2,5 m', manufacturer: 'Global Truss', manufacturer_no: 'F32-250', retail_price: 189.0, weight: 7.2, width: 29, height: 29, depth: 250, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: true, devices: 20, short: 'Alu-Traverse F32, 2,5 m, 3-Punkt', long: 'Standardtraverse für Bühnendächer und Lichtbrücken, koppelbar über Konusverbinder.', technical: 'F32, 2,5 m, 29 cm, Alu EN-AW 6082 T6, 7,2 kg' },
	{ group: 'Bühne', name: 'Traversenlift 6 m', manufacturer: 'Work', manufacturer_no: 'LW-6000', retail_price: 1690.0, weight: 32.0, width: 30, height: 200, depth: 30, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: true, devices: 4, short: 'Lift für Traversen, 6 m Arbeitshöhe', long: 'Handkurbel-Lift zum Aufbau von Lichtbrücken und Traversenkreuzen. TÜV-geprüfte Ausführung mit Lastsicherung.', technical: 'Arbeitshöhe 6 m, Traglast 150 kg, 32 kg' },
	{ group: 'Bühne', name: 'Kettenzug 1000 kg', manufacturer: 'Liftket', manufacturer_no: 'STAR-1000', retail_price: 890.0, weight: 15.0, width: 25, height: 30, depth: 20, power: 1100, voltage: [230, 230], current: 5.0, inspection_class: 1, inspection_months: 12, individual: true, devices: 8, short: 'Elektrokettenzug 1000 kg, 6 m Hub', long: 'Elektrokettenzug zum Fliegen von Lasten im Bühnenbereich. Nur durch befähigte Person zu bedienen, jährliche Prüfung.', technical: '1000 kg, 6 m Hub, 400 V optional, 15 kg' },
	{ group: 'Bühne', name: 'Bühnenpodest 2×1 m', manufacturer: 'Nivtec', manufacturer_no: 'POD-200-100', retail_price: 640.0, weight: 28.0, width: 200, height: 20, depth: 100, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: true, devices: 24, short: 'Podestelement 2,00 × 1,00 m, 20 cm', long: 'Modulares Bühnensystem mit Stecksystem und höhenverstellbaren Füßen. Belag Rutschhemmend, Systemzulassung beachten.', technical: '2000 × 1000 × 200 mm, Traglast 750 kg/m², Systemnivtec' },
	{ group: 'Bühne', name: 'Bühnenvorhang 8×4 m', manufacturer: 'Gerriets', manufacturer_no: 'MOLTON-800-400', retail_price: 1290.0, weight: 22.0, width: 800, height: 400, depth: 5, power: null, voltage: null, current: null, inspection_class: 1, inspection_months: 24, individual: true, devices: 3, short: 'Moltonvorhang, schwer entflammbar, 8 × 4 m', long: 'Moltonvorhang zur Abhängung von Bühnenrückwänden. Schwer entflammbar nach DIN 4102 B1, Nachweis im Ordner.', technical: '800 × 400 cm, Molton 300 g/m², B1, Ösenband alle 25 cm' },
	{ group: 'Bühne', name: 'Rigging-Rahmen 4-Punkt', manufacturer: 'Global Truss', manufacturer_no: 'F34-GRID', retail_price: 540.0, weight: 9.5, width: 50, height: 50, depth: 8, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: true, devices: 2, short: 'Rahmen für Bodenaufbau und Flug', long: 'Vierpunkt-Rigging-Rahmen passend zur F34-Traverse, für Bodenaufbau mit Ballast oder zum Fliegen.', technical: 'F34, 500 × 500 mm, 9,5 kg' },

	// ------------------------------------------------------------------ Video
	{ group: 'Video', name: 'Beamer 10.000 ANSI-Lumen', manufacturer: 'Panasonic', manufacturer_no: 'PT-RZ10K', retail_price: 18900.0, weight: 22.0, width: 50, height: 20, depth: 45, power: 950, voltage: [100, 240], current: 9.0, inspection_class: 1, inspection_months: 12, individual: true, devices: 2, for_sale: false, short: '1-Chip-DLP-Projektor WUXGA, 10.000 lm', long: 'Laserprojektor für große Projektionsflächen. WUXGA, motorischer Zoom, Lens-Shift, Netzwerksteuerung.', technical: '10.000 lm, WUXGA 1920×1200, Laser-Phosphor, 22 kg' },
	{ group: 'Video', name: 'LED-Wall Panel 500×500 mm', manufacturer: 'Absen', manufacturer_no: 'PL2.9', retail_price: 1290.0, weight: 8.0, width: 50, height: 50, depth: 8, power: 180, voltage: [100, 240], current: 1.5, inspection_class: 2, inspection_months: 12, individual: true, devices: 30, short: 'LED-Panel 2,9 mm Pitch, 500 × 500 mm', long: 'Feinpixel-LED-Panel für Bühnenbild und Präsentation. Verriegelungssystem, Netzteil- und Datenredundanz.', technical: 'Pitch 2,9 mm, 168×168 px, 180 W, 8 kg' },
	{ group: 'Video', name: 'Videoswitcher 8 Eingänge', manufacturer: 'Blackmagic Design', manufacturer_no: 'ATEM-TV-STUDIO-HD8', retail_price: 2950.0, weight: 3.5, width: 43, height: 9, depth: 25, power: 60, voltage: [100, 240], current: 0.6, inspection_class: 2, inspection_months: 12, individual: true, devices: 2, for_sale: false, short: 'Mischer mit 8 SDI-Eingängen, ISO-Aufzeichnung', long: 'Live-Videomischer für Konferenzen und Streaming. 8 SDI-Eingänge, Multiview, USB-Disk-Aufzeichnung, Streaming-Engine.', technical: '8× 12G-SDI, 2× HDMI, Multiview, 3,5 kg' },
	{ group: 'Video', name: 'PTZ-Kamera 20× Zoom', manufacturer: 'Panasonic', manufacturer_no: 'AW-UE20', retail_price: 2490.0, weight: 1.6, width: 18, height: 17, depth: 19, power: 25, voltage: [100, 240], current: 0.2, inspection_class: 2, inspection_months: 12, individual: true, devices: 3, short: 'PTZ-Kamera mit 20× Zoom und NDI', long: 'Schwenk-Neige-Zoom-Kamera für Konferenz- und Hybridveranstaltungen. NDI|HX, PoE+, Fernsteuerung.', technical: '20× optischer Zoom, 4K 25p, NDI|HX, PoE+, 1,6 kg' },
	{ group: 'Video', name: 'Projektionsleinwand 6×4 m', manufacturer: 'Gerriets', manufacturer_no: 'CINEMA-600-400', retail_price: 1480.0, weight: 14.0, width: 620, height: 420, depth: 10, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: true, devices: 2, short: 'Rahmenleinwand, Bildformat 3:2', long: 'Flugfähige Projektionsleinwand mit Tuchspannung über Keder, für Front- und Rückprojektion geeignet.', technical: '600 × 400 cm, Tuch Gain 1,0, Rahmen Alu, 14 kg' },
	{ group: 'Video', name: 'Medienserver kompakt', manufacturer: 'Green Hippo', manufacturer_no: 'HIPPOTIZER', retail_price: 8900.0, weight: 6.0, width: 43, height: 9, depth: 35, power: 300, voltage: [100, 240], current: 2.0, inspection_class: 2, inspection_months: 12, individual: true, devices: 1, for_sale: false, short: 'Medienserver für Zuspielung und Mapping', long: 'Kompakter Medienserver für Mehrprojektor-Setups und Mapping. Synchrone Ausgabe über NDI und SDI.', technical: '2× 4K-Ausgabe, NDI, 1 HE, 6 kg' },

	// ----------------------------------------------------------- Consumables
	{ group: 'Verbrauchsmaterial', name: 'Gaffa-Tape 50 mm schwarz', manufacturer: 'Gerband', manufacturer_no: 'GAFFA-50-SW', retail_price: 19.9, weight: 0.4, width: 5, height: 11, depth: 11, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: false, for_sale: true, no_own_stock: true, short: 'Gewebeband 50 mm × 50 m, rückstandsfrei', long: 'Mattschwarzes Gewebeband zum Abkleben von Kabeln und Kanten. Rückstandsfrei ablösbar, auch auf Bühnenboden.', technical: '50 mm × 50 m, Naturkautschuk, mattschwarz' },
	{ group: 'Verbrauchsmaterial', name: 'Leuchtmittel HPL 750 W', manufacturer: 'Osram', manufacturer_no: 'HPL-750-230', retail_price: 24.5, weight: 0.1, width: 5, height: 9, depth: 5, power: 750, voltage: [230, 230], current: 3.3, inspection_class: null, inspection_months: null, individual: false, for_sale: true, no_own_stock: true, short: 'HPL-Lampe 750 W, 230 V, 4-fach gewendelt', long: 'Ersatzleuchtmittel für Profilscheinwerfer. Nur mit Handschuhen einsetzen, Fingerabdrücke verkürzen die Lebensdauer.', technical: '750 W, 230 V, Sockel G9.5, 1150 h' },

	// ---------------------------------------------------------------- Effects
	{ supplierNos: [2005], group: 'Effekte', name: 'Nebelmaschine 1500 W', manufacturer: 'Look Solutions', manufacturer_no: 'VIPER-NT', retail_price: 1290.0, weight: 9.5, width: 45, height: 20, depth: 30, power: 1500, voltage: [230, 230], current: 6.8, inspection_class: 1, inspection_months: 12, individual: true, devices: 3, for_sale: false, short: 'Nebelmaschine mit DMX und Timer', long: 'Nebelmaschine für Bühnennebel, Ausstoß über DMX steuerbar, Aufheizzeit kurz. Nur mit freigegebenem Nebelfluid betreiben.', technical: '1500 W, Aufheizzeit 6 s, Ausstoß 120 m³/min, DMX 512, 9,5 kg' },
	{ group: 'Effekte', name: 'Konfettimaschine CO2', manufacturer: 'MagicFX', manufacturer_no: 'CONFETTI-CO2', retail_price: 890.0, weight: 6.0, width: 30, height: 40, depth: 30, power: null, voltage: null, current: null, inspection_class: 1, inspection_months: 6, individual: true, devices: 2, for_sale: false, short: 'Konfettikanone für CO2-Betrieb', long: 'Effektgerät für Konfettischüsse, Betrieb mit CO2-Flaschen. Aufstellbereich und Sicherheitsabstände nach Herstellerangabe.', technical: 'Reichweite 12 m, CO2-Flasche 2 kg, 6 kg' },
	{
		group: 'Effekte',
		name: 'Bühnenpyro-Effektkörper 1.4G, 12er-Pack',
		manufacturer: 'Pyro-Shop',
		manufacturer_no: 'PYRO-14G-12',
		retail_price: 189.0,
		weight: 1.8,
		width: 30,
		height: 12,
		depth: 20,
		power: null,
		voltage: null,
		current: null,
		inspection_class: 1,
		inspection_months: 6,
		individual: false,
		for_sale: false,
		no_own_stock: true,
		short: 'Effektkörper Klasse 1.4G, VE 12',
		long: 'Pyrotechnische Effektkörper für Bühnenanwendungen. Erwerb und Verwendung nur mit Erlaubnis nach SprengG; Lagerung nur in zugelassenen Behältern.',
		technical: 'UN 0336, Klasse 1.4G, NEM ca. 12 g je Stück, VE 12',
		// Register „Pyro": die Werte, nach denen die Artikelsuche filtert.
		pyro: {
			un_number: '0336',
			un_description: 'Feuerwerkskörper, Klasse 1.4G',
			hazard_class: '1.4G',
			caliber: 50,
			lateral_safety_distance: '8.0',
			vertical_safety_distance: '12.0',
			ascending_height: '25.0',
			sound_pressure_level: '120.0',
			sound_pressure_range: '15.0',
			tunnel_restriction_code: 'D',
			delay_time: '2.5',
			effect_duration: '6.0',
			category: 'Kategorie 2',
			bam_id: 'BAM-P1-1234',
			classification: 'PM-II',
			packing_group: 'II',
			approval_number: '4-1234-22',
			neq: '1.25',
			neq_unit: 'kg',
			un_points: '60',
			transport_category: '3'
		}
	},

	// -------------------------- inactive, virtual article, no own stock
	{ group: 'Licht', name: 'PAR-Scheinwerfer 64 (Altbestand)', manufacturer: 'Thomas', manufacturer_no: 'PAR-64-ALT', retail_price: 0, weight: 2.0, width: 21, height: 24, depth: 21, power: null, voltage: null, current: null, inspection_class: 1, inspection_months: 12, individual: true, devices: 0, active: false, short: 'Klassischer PAR-64, ausgemustert', long: 'Altbestand an PAR-64-Gehäusen, wird nicht mehr disponiert und ist nur zur Nachvollziehbarkeit von Altdaten geführt.' },
	{ group: 'Licht', name: 'Stufenlinsen-Scheinwerfer 1 kW (Altbestand)', manufacturer: 'Strand', manufacturer_no: 'SL-1000-ALT', retail_price: 0, weight: 6.5, width: 30, height: 33, depth: 30, power: 1000, voltage: [230, 230], current: 4.5, inspection_class: 1, inspection_months: 12, individual: true, devices: 0, active: false, short: 'Alter Stufenlinser, nicht mehr im Umlauf', long: 'Ausgemustertes Gerät. Bleibt mit Historie im Stamm, damit alte Belege zuordenbar bleiben.' },
	{ group: 'Bühne', name: 'Rigging-Planung (Dienstleistung)', manufacturer: null, manufacturer_no: null, retail_price: 480.0, weight: null, width: null, height: null, depth: null, power: null, voltage: null, current: null, inspection_class: null, inspection_months: null, individual: false, virtual: true, short: 'Planungsleistung, kein physischer Bestand', long: 'Statische Prüfung und Rigging-Plan durch eine befähigte Person. Wird als Dienstleistungsposition abgerechnet, kein Lagerbestand.' }
];

const defaultStockId = await (async () => {
	// The default warehouse shows up in every stock level; it needs one article.
	const firstPage = await api('/api/v1/articles?limit=1');
	const articleId = firstPage.items[0]?.id;
	if (!articleId) return 1;
	const rows = (await api(`/api/v1/articles/${articleId}/stock`)).stock;
	return (rows.find((r) => r.is_default) ?? rows[0])?.stock_id ?? 1;
})();
const today = new Date().toISOString().slice(0, 10);

/** Lieferanten-Adressen nach Lieferantennummer — Grundlage des Registers „Lieferanten". */
const lieferanten = await (async () => {
	try {
		const page = await api('/api/v1/addresses/search', {
			method: 'POST',
			body: JSON.stringify({ address_type: 'suppliers', limit: 500 })
		});
		return new Map(
			page.items.map((x) => [
				x.supplier_no ?? x.customer_no,
				{ id: x.id, no: x.supplier_no ?? x.customer_no ?? null, name: x.name1 ?? null }
			])
		);
	} catch (error) {
		console.warn(`  suppliers not available: ${error.message}`);
		return new Map();
	}
})();

/** Lieferanten eines Artikels aus `supplierNos` (Nummern der Adressen). */
function suppliersOf(a) {
	return (a.supplierNos ?? [])
		.map((no, index) => {
			const treffer = lieferanten.get(no);
			if (!treffer) return null;
			return {
				supplier_id: treffer.id,
				supplier_no: treffer.no,
				name: treffer.name,
				position: index + 1,
				// Einkaufspreis grob aus dem UVP, damit die Liste plausibel aussieht.
				purchase_price: a.retail_price ? String(Math.round(a.retail_price * 0.6 * 100) / 100) : null,
				delivery_days: 7 + index * 3,
				favorite: index === 0,
				sales_article: false,
				supplier_article_no: a.manufacturer_no ? `${a.manufacturer_no}-${treffer.no ?? index}` : null
			};
		})
		.filter(Boolean);
}

let created = 0;
let updated = 0;
let skipped = 0;
let devicesCreated = 0;
let eanCounter = 200000000001;

for (const a of ARTICLES) {
	const bekannt = existing.get(a.name.trim().toLowerCase());
	if (bekannt) {
		// Der Satz existiert schon: Warengruppen, Aktiv-Kennzeichen und die neuen
		// Register nachziehen, damit ein älterer Lauf zum aktuellen Stand kommt.
		const gespeichert = await api(`/api/v1/articles/${bekannt}`);
		const soll = groupsOf(a);
		const ist = [...(gespeichert.article_groups ?? [])].sort((x, y) => x - y);
		const sollAktiv = a.active ?? true;
		const sollLieferanten = suppliersOf(a);
		const istLieferanten = (gespeichert.suppliers ?? []).map((s) => s.supplier_id).sort();
		const pyroFehlt = a.pyro && !gespeichert.pyro?.un_number;
		const lieferantenFehlt =
			sollLieferanten.length > 0 &&
			sollLieferanten.map((s) => s.supplier_id).sort().join(',') !== istLieferanten.join(',');
		if (
			soll.join(',') !== ist.join(',') ||
			gespeichert.active !== sollAktiv ||
			pyroFehlt ||
			lieferantenFehlt
		) {
			await api(`/api/v1/articles/${bekannt}`, {
				method: 'PUT',
				body: JSON.stringify({
					...gespeichert,
					article_groups: soll,
					active: sollAktiv,
					pyro: a.pyro ?? gespeichert.pyro,
					suppliers: lieferantenFehlt ? sollLieferanten : (gespeichert.suppliers ?? [])
				})
			});
			updated++;
			console.log(`  updated: ${a.name}`);
		} else {
			skipped++;
			console.log(`  skipped: ${a.name}`);
		}
		continue;
	}
	const deviceCount = args.devices ? (a.devices ?? 0) : 0;
	const inventoryNo = `${String(group[a.group]).padStart(2, '0')}-${String(created + 1).padStart(4, '0')}`;
	const body = {
		...template,
		id: null,
		article_no: 0,
		name: a.name,
		short_name: a.short?.slice(0, 60) ?? null,
		match_code: a.name.replace(/[^A-Za-zÄÖÜäöüß0-9]/g, '').slice(0, 20).toUpperCase(),
		unit: 'Stück',
		inventory_no: inventoryNo,
		active: a.active ?? true,
		inventory: a.individual ?? false,
		rental: true,
		sale: a.for_sale ?? false,
		stock_tracking: true,
		no_own_stock: a.no_own_stock ?? false,
		individual_items: a.individual ?? false,
		virtual_article: a.virtual ?? false,
		ean_code: ean13(eanCounter++),
		manufacturer_no: a.manufacturer_no ?? null,
		manufacturer: a.manufacturer ?? null,
		manufacturer_country: a.manufacturer ? 'DE' : null,
		retail_price: a.retail_price ?? null,
		customs_tariff_export: a.group === 'Licht' || a.group === 'Video' ? '9405 4010' : null,
		insurance_value_net: a.retail_price ? Math.round(a.retail_price * 0.8 * 100) / 100 : null,
		team_group:
			a.group === 'Ton' ? 'Ton-Team' : a.group === 'Licht' ? 'Licht-Team' : 'Technik-Team',
		width_net: a.width ?? null,
		height_net: a.height ?? null,
		depth_net: a.depth ?? null,
		weight_net: a.weight ?? null,
		weight_gross: a.weight ? Math.round((a.weight + 1.5) * 100) / 100 : null,
		voltage_from: a.voltage?.[0] ?? null,
		voltage_to: a.voltage?.[1] ?? null,
		current_draw: a.current ?? null,
		power: a.power ?? null,
		setup_minutes: (a.devices ?? 0) > 10 ? 45 : null,
		teardown_minutes: (a.devices ?? 0) > 10 ? 30 : null,
		check_out_seconds: a.individual ? 90 : null,
		check_in_seconds: a.individual ? 120 : null,
		inspection_class: a.inspection_class ?? null,
		inspection_interval_months: a.inspection_months ?? null,
		inspection_minutes: a.inspection_months ? 10 : null,
		workshop_by_months: Boolean(a.inspection_months),
		workshop_months: a.inspection_months ?? null,
		texts: {
			short: a.short ?? null,
			long: a.long ?? null,
			technical_details: a.technical ?? null,
			scope_of_delivery: a.scope ?? null,
			packaging_info: a.scope ? '1 Karton' : null,
			webshop: null
		},
		article_groups: groupsOf(a),
		suppliers: suppliersOf(a),
		pyro: { ...(template.pyro ?? {}), ...(a.pyro ?? {}) }
	};

	// Tabs "Miete" and "Verkauf": prices derived from the retail price so the form shows
	// plausible values. The three extra price variants are switched on for every third
	// article so all sub-tabs of the rental tab have data.
	const retail = a.retail_price ?? 0;
	const net = retail ? Math.round((retail / 1.19) * 100) / 100 : null;
	const variantOn = created % 3 === 0;
	body.pricing = {
		...template.pricing,
		rental_price_per_day: retail ? Math.round(retail * 0.02 * 100) / 100 : null,
		rental_base_price: retail ? Math.round(retail * 0.01 * 100) / 100 : null,
		rental_cost_price: retail ? Math.round(retail * 0.6 * 100) / 100 : null,
		deposit: retail ? Math.round(retail * 0.3 * 100) / 100 : null,
		rental_price_per_day_roll_days: variantOn && retail ? Math.round(retail * 0.019 * 100) / 100 : null,
		rental_price_per_day_off_days: variantOn && retail ? Math.round(retail * 0.021 * 100) / 100 : null,
		rental_price_weekend: variantOn && retail ? Math.round(retail * 0.05 * 100) / 100 : null,
		rental_cost_price_weekend: variantOn && retail ? Math.round(retail * 0.03 * 100) / 100 : null,
		rental_roll_days_active: variantOn,
		rental_off_days_active: variantOn,
		rental_weekend_active: variantOn,
		used_sale_price: retail ? Math.round(retail * 0.45 * 100) / 100 : null,
		rental_vat_rate: 19,
		rental_revenue_account: '8400',
		rental_cost_center: 'KST-MIETE',
		capital_binding: retail ? Math.round(retail * 0.8 * 100) / 100 : null,
		approximate_location: `Regal ${String.fromCharCode(65 + (created % 6))}`,
		factor_scale: 1,
		checkout_requires_serial: a.individual ?? false,
		unique_article: a.individual ?? false,
		fixed_installation: a.group === 'Bühne',
		material_only: a.group === 'Verbrauchsmaterial',
		inspection_on_checkin: Boolean(a.inspection_months),
		sale_price_net: net,
		sale_price_gross: retail || null,
		sale_retail_price: retail || null,
		sale_cost_price_net: net ? Math.round(net * 0.6 * 100) / 100 : null,
		sale_markup_percent: 66.67,
		sale_vat_rate: 19,
		sale_revenue_account: '8401',
		sale_cost_center: 'KST-VK',
		quantity_scale: a.for_sale ? 2 : null,
		sell_full_packages: a.group === 'Verbrauchsmaterial',
		package_unit: a.group === 'Verbrauchsmaterial' ? 'Karton' : null,
		package_size: a.group === 'Verbrauchsmaterial' ? 10 : null,
		reorder_level: a.for_sale ? 5 : null
	};
	const stored = await api('/api/v1/articles', { method: 'POST', body: JSON.stringify(body) });
	created++;
	const serials = [];
	for (let i = 1; i <= deviceCount; i++) {
		const serialNo = `${(a.manufacturer_no ?? 'SN').slice(0, 10)}-${String(i).padStart(3, '0')}`;
		try {
			await api(`/api/v1/articles/${stored.id}/devices`, {
				method: 'POST',
				body: JSON.stringify({
					serial_no: serialNo,
					stock_id: defaultStockId,
					bin_location: `Regal ${String.fromCharCode(65 + ((created + i) % 6))}${1 + ((created + i) % 4)}`,
					condition: 'gebraucht, geprüft',
					purchase_price: a.retail_price ? Math.round(a.retail_price * 0.6 * 100) / 100 : null,
					purchase_date: today,
					inventory_no: `${inventoryNo}-${String(i).padStart(2, '0')}`,
					inventory: true,
					rental: true,
					sale: a.for_sale ?? false,
					used_sale: false
				})
			});
			devicesCreated++;
			serials.push(serialNo);
		} catch (error) {
			console.warn(`     device ${serialNo}: ${error.message}`);
		}
	}
	console.log(
		`  created: ${stored.article_no}  ${a.name}${serials.length ? `  (+${serials.length} devices)` : ''}`
	);
}

	console.log(
	`\ndone: ${created} articles created, ${updated} updated, ${skipped} skipped, ${devicesCreated} devices received.`
	);
