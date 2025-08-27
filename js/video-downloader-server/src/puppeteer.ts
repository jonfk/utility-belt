import puppeteer, { Browser } from 'puppeteer';
import { config } from './config.js';

let browser: Browser | null = null;

export async function getBrowser(): Promise<Browser> {
	if (!browser) {
		browser = await puppeteer.launch({
			headless: true,
		});
	}
	return browser;
}

export async function closeBrowser(): Promise<void> {
	if (browser) {
		await browser.close();
		browser = null;
	}
}
