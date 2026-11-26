/**
 * The English dictionary. Every entry is keyed by the German text that stands in
 * the code; `t()` looks the key up here when the interface runs in English.
 *
 * The entries are split by area so that a text belongs to exactly one file: the
 * shell (menu, sign-in, common words), the two halves of the core, and the three
 * modules. A key that is missing simply stays German.
 */
import { shell } from './en/shell';
import { corelib } from './en/core-lib';
import { coreroutes } from './en/core-routes';
import { people } from './en/people';
import { go } from './en/go';
import { money } from './en/money';
import { backend } from './en/backend';

export type Entry = string | { one: string; other: string };

export const en: Record<string, Entry> = {
	...shell,
	...corelib,
	...coreroutes,
	...people,
	...go,
	...money,
	...backend
};
