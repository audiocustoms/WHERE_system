import {
	api,
	errorText,
	setAuthToken,
	setUnauthorizedHandler,
	type LoginTenant,
	type LoginUser,
	type SessionUser
} from './api';

/**
 * The session of the application.
 *
 * The token lives in `sessionStorage`: it survives a reload of the page, but not
 * the closing of the window or of the desktop application — a new start therefore
 * always begins with the login mask.
 */
const STORAGE_KEY = 'where.session';

/**
 * The right behind the `Admin` menu: `Zugriff erlauben` of the administrative
 * settings in the right tree of the user groups. Must match
 * `where_domain::ADMIN_RIGHT`.
 */
export const ADMIN_RIGHT = 'administrative-einstellungen.zugriff';

class Auth {
	token = $state<string | null>(null);
	user = $state<SessionUser | null>(null);
	/** The mask waits until the stored session has been verified. */
	restoring = $state(true);
	error = $state<string | null>(null);
	load = $state(false);

	/** `true` while an account is signed in. */
	get signedIn(): boolean {
		return this.user !== null;
	}

	/** The name of the header line: `Administrator [Adm]`. */
	get label(): string {
		const user = this.user;
		if (!user) return '';
		const name = user.display_name?.trim() || user.user_name;
		return user.short_name?.trim() ? `${name} [${user.short_name.trim()}]` : name;
	}

	/** The rights of the account from its permission groups. */
	get rights(): string[] {
		return this.user?.rights ?? [];
	}

	/** May the account do this one thing? Keys such as `artikel.artikel-anlegen`. */
	hasRight(key: string): boolean {
		return this.rights.includes(key);
	}

	/**
	 * The `Admin` menu of the taskbar and all settings pages hang on this right —
	 * the checkbox `Zugriff erlauben` of the `administrativen Einstellungen`.
	 */
	get isAdmin(): boolean {
		return this.hasRight(ADMIN_RIGHT);
	}

	/** On start: verify a token from `sessionStorage` against the server. */
	async restore(): Promise<void> {
		this.restoring = true;
		this.error = null;
		try {
			const saved =
				typeof sessionStorage === 'undefined' ? null : sessionStorage.getItem(STORAGE_KEY);
			if (!saved) return;
			setAuthToken(saved);
			this.user = await api.auth.session();
			this.token = saved;
		} catch {
			// An expired or unknown token leads back to the mask.
			this.forgot();
		} finally {
			this.restoring = false;
		}
	}

	async login(userName: string, password: string, mandant: number | null = null): Promise<boolean> {
		this.load = true;
		this.error = null;
		try {
			const answer = await api.auth.login({ user_name: userName, password, mandant });
			this.token = answer.token;
			this.user = answer.user;
			setAuthToken(answer.token);
			if (typeof sessionStorage !== 'undefined') sessionStorage.setItem(STORAGE_KEY, answer.token);
			return true;
		} catch (e) {
			this.error = errorText(e);
			return false;
		} finally {
			this.load = false;
		}
	}

	/** Sign out: end the session on the server, forget the token in the window. */
	async logout(): Promise<void> {
		try {
			if (this.token) await api.auth.logout();
		} catch {
			// Even without a server answer the session ends in the window.
		} finally {
			this.forgot();
		}
	}

	/** Forget locally only — without a server call. */
	forgot(): void {
		this.token = null;
		this.user = null;
		setAuthToken(null);
		if (typeof sessionStorage !== 'undefined') sessionStorage.removeItem(STORAGE_KEY);
	}

	/** The password is still the one of the initial installation. */
	get mustChangePassword(): boolean {
		return this.user?.change_next_login === true;
	}

	/**
	 * Set a new password. Work is allowed afterwards; the application no longer
	 * shows the mask of the first login.
	 */
	async changePassword(password: string, repeat: string): Promise<boolean> {
		this.load = true;
		this.error = null;
		try {
			await api.auth.changePassword({ password, repeat });
			if (this.user) this.user = { ...this.user, change_next_login: false };
			return true;
		} catch (e) {
			this.error = errorText(e);
			return false;
		} finally {
			this.load = false;
		}
	}

	/** The accounts of the selection list of a tenant. */
	async accounts(mandant: number): Promise<LoginUser[]> {
		const list = await api.auth.users(mandant);
		return list.sort((a, b) => {
			const aHat = a.has_password ? 0 : 1;
			const bHat = b.has_password ? 0 : 1;
			if (aHat !== bHat) return aHat - bHat;
			return (a.display_name ?? a.user_name).localeCompare(b.display_name ?? b.user_name, 'de');
		});
	}

	/** The tenants for the selection left next to `Benutzer`. */
	async tenants(): Promise<LoginTenant[]> {
		return api.auth.tenants();
	}

	/**
	 * Tenant switch while running: the session moves into the other tenant, and
	 * because the tenant determines the database, the application reloads afterwards.
	 */
	async changeTenant(mandant: number): Promise<boolean> {
		if (this.user?.mandant === mandant) return true;
		this.load = true;
		this.error = null;
		try {
			this.user = await api.auth.switchMandant(mandant);
			if (typeof localStorage !== 'undefined' && this.user) {
				// The login mask remembers the last choice (`where.last-login`).
				localStorage.setItem(
					'where.last-login',
					JSON.stringify({ mandant, user_name: this.user.user_name })
				);
			}
			return true;
		} catch (e) {
			this.error = errorText(e);
			return false;
		} finally {
			this.load = false;
		}
	}
}

export const auth = new Auth();

// A session rejected by the server (401) ends the login in the window, the mask
// appears again — without every page having to show its own error.
setUnauthorizedHandler(() => {
	if (auth.user) auth.forgot();
});
