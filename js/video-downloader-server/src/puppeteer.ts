import puppeteer, { Browser } from 'puppeteer';
import { config } from './config.js';

let browser: Browser | null = null;

export async function getBrowser(): Promise<Browser> {
	if (!browser) {
		const args = ['--no-sandbox', '--disable-dev-shm-usage'];
		
		browser = await puppeteer.launch({
			headless: true,
			args,
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
