/** The open state of the personal settings — a window like the masks. */
let open = $state(false);

export const personalSettings = {
	get open(): boolean {
		return open;
	},
	show(): void {
		open = true;
	},
	hide(): void {
		open = false;
	}
};
