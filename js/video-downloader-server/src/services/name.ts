import moment from 'moment';
import { sanitizeFilename } from '../storage.js';
import { UnsupportedUrlError, NameResolutionError } from '../errors.js';
import { getBrowser } from '../puppeteer.js';

export interface NameResolver {
  resolveName(url: string): Promise<string>;
}

export class SxyPrnNameResolver implements NameResolver {
  async resolveName(url: string): Promise<string> {
    const browser = await getBrowser();
    const page = await browser.newPage();
    
    try {
      await page.goto(url, { waitUntil: 'networkidle0' });
      const title = await page.title();
      const processedTitle = await this.processTitle(title);
      return processedTitle;
    } catch (error) {
      throw new NameResolutionError(`Failed to resolve name for URL: ${url}`, { 
        originalError: error instanceof Error ? error.message : String(error) 
      });
    } finally {
      await page.close();
    }
  }

  private async processTitle(title: string, prefix: string = ''): Promise<string> {
    if (prefix) {
      title = `${prefix} ${title}`;
    }
    
    const promotionalPatterns = [
      /\[[^\]]*(?:backup|visit|watch)[^\]]*\]/i,
      /(?:backup|watch|download)(?:\s*\/\s*(?:watch|download))?\s*(?:hd|fhd|full\s*hd)(?:\s*:)?/i,
      /backup\s+(?:hd|fhd|full\s*hd)\s+on\s+link/i,
      /(?:also\s+)?visit\s+(?:my\s+)?(?:blog|website)\s*[-:]?\s*(?:https?:\/\/[^\s]+)?(?:\s+for\s+backup[^a-z]+)/i,
      /watch\s+online\s+(?:hd|fhd|full\s*hd)(?:\s*:)?/i
    ];
    
    const cleanupPatterns = [
      /\b(?:720p?|1080p?|1440p|2160p|480p|360p)\b/gi,
      /\b(?:4k|8k|hd|uhd|qhd|full\s*hd)(?:\s+quality)?\b/gi,
      /\b(?:@premium|@verified|#verified|#premium)\b/gi,
      /(?:^|\s)#?(?:ghost|internallink|link|dailyvids|0dayporn|freeporn|premiumcontent)\b/gi,
      /\b(?:xxx|adultvideo|porn(?:video)?)\b/gi,
      /\b(?:click\s*here|subscribe\s*now|stream\s*here)\b/gi
    ];
    
    promotionalPatterns.forEach(pattern => {
      title = title.replace(pattern, '');
    });
    
    cleanupPatterns.forEach(pattern => {
      title = title.replace(pattern, '');
    });
    
    title = title.replace(/(?:https?|ftp):\/\/[\n\S]+/g, '');
    title = title.replace(/\s+/g, ' ');
    title = title.trim();
    
    let dateMatch = title.match(/\b\d{4}[-./]\d{1,2}[-./]\d{1,2}\b|\b\d{1,2}[-./]\d{1,2}[-./]\d{4}\b/);
    let extractedDate = '';
    
    if (dateMatch) {
      const parsedDate = moment(dateMatch[0], ['YYYY-MM-DD', 'DD-MM-YYYY', 'MM-DD-YYYY', 'DD.MM.YYYY', 'YYYY/MM/DD', 'YYYY.MM.DD', 'YY MM DD'], true);
      if (parsedDate.isValid()) {
        extractedDate = parsedDate.format('YYYY-MM-DD');
        title = title.replace(dateMatch[0], '').trim();
      }
    }
    
    title = title.substring(0, 200);
    
    if (extractedDate) {
      title = `${extractedDate}_${title}`;
    }
    
    return sanitizeFilename(title);
  }
}

export class DefaultNameResolver implements NameResolver {
  private sxyPrnResolver = new SxyPrnNameResolver();

  async resolveName(url: string): Promise<string> {
    const urlObj = new URL(url);
    
    if (urlObj.hostname.includes('sxyprn.com')) {
      return this.sxyPrnResolver.resolveName(url);
    }
    
    throw new UnsupportedUrlError(`URL hostname not supported: ${urlObj.hostname}`);
  }
}