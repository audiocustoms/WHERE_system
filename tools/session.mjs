// Access for the seed scripts.
//
// Every route except the login mask asks for a session and a right (see README,
// "Rechte"), so the seeds either take a token or sign in with an account:
//
//   node tools/seed-articles.mjs --user admin2 --password start --mandant 3
//   node tools/seed-articles.mjs --token 0123…            (or WHERE_TOKEN)
//
// `--mandant` is the tenant whose database receives the data; without it the
// installation's own database is used.

/** Reads `--token`, `--user`, `--password` and `--mandant`, each also from the environment. */
export function sessionArgs(argv = process.argv) {
	const args = {
		token: process.env.WHERE_TOKEN ?? '',
		user: process.env.WHERE_USER ?? '',
		password: process.env.WHERE_PASSWORD ?? '',
		mandant: process.env.WHERE_MANDANT ?? ''
	};
	for (let i = 2; i < argv.length; i++) {
		const value = argv[i + 1] ?? '';
		if (argv[i] === '--token') args.token = value;
		else if (argv[i] === '--user') args.user = value;
		else if (argv[i] === '--password') args.password = value;
		else if (argv[i] === '--mandant') args.mandant = value;
		else continue;
		i++;
	}
	return args;
}

/**
 * The `Authorization` header — from `--token`, otherwise from a sign-in with the
 * given account. Throws when neither is configured, because no route is open.
 */
export async function sessionHeader(api, args = sessionArgs()) {
	if (args.token) return { authorization: `Bearer ${args.token}` };
	if (!args.user) {
		throw new Error(
			'Kein Zugang: --token <token> oder --user <name> --password <kennwort> [--mandant <id>] angeben ' +
				'(oder WHERE_TOKEN / WHERE_USER / WHERE_PASSWORD setzen).'
		);
	}
	let mandant = null;
	if (args.mandant !== '') {
		mandant = Number(args.mandant);
		if (!Number.isInteger(mandant)) {
			throw new Error(`--mandant muss eine Zahl sein, gefunden: "${args.mandant}"`);
		}
	}
	const response = await fetch(`${api}/api/v1/auth/login`, {
		method: 'POST',
		headers: { 'content-type': 'application/json' },
		body: JSON.stringify({ user_name: args.user, password: args.password, mandant })
	});
	if (!response.ok) {
		throw new Error(`Anmeldung als "${args.user}" fehlgeschlagen (${response.status}): ${await response.text()}`);
	}
	const body = await response.json();
	const wo = body.user?.mandant_name ? ` in ${body.user.mandant_name}` : ' (Installationsdatenbank)';
	console.log(`angemeldet als ${body.user?.user_name ?? args.user}${wo}`);
	return { authorization: `Bearer ${body.token}` };
}
