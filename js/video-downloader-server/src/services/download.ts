import type { CompletedDownload } from '../schemas.js';
import puppeteer from 'puppeteer';
import fs from 'fs';
import path from 'path';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';
import type { ReadableStream as NodeReadableStream } from 'stream/web';

export interface DownloadJob {
  jobId: string;
  url: string;
  name: string;
  enqueuedAt: Date;
}

interface Downloader {
  download(url: string, name: string): Promise<CompletedDownload>;
}

class SxyPrnDownloader implements Downloader {
  async download(url: string, name: string): Promise<CompletedDownload> {
    const browser = await puppeteer.launch({ headless: true });
    const page = await browser.newPage();
    
    try {
      await page.goto(url, { waitUntil: 'networkidle0' });
      await page.waitForSelector('#player_el');
      
      const videoSrc = await page.$eval('#player_el', el => (el as HTMLVideoElement).src);
      if (!videoSrc) {
        throw new Error('Could not find video source');
      }
      
      const canonicalUrl = await this.getCanonicalUrl(videoSrc);
      const outputPath = path.join(process.cwd(), 'data', `${name}.mp4`);
      
      await fs.promises.mkdir(path.dirname(outputPath), { recursive: true });
      
      const startedAt = new Date();
      const size = await this.downloadVideo(canonicalUrl, outputPath);
      const finishedAt = new Date();
      
      return {
        url,
        name,
        savedPath: outputPath,
        size,
        startedAt: startedAt.toISOString(),
        finishedAt: finishedAt.toISOString()
      };
    } finally {
      await page.close();
      await browser.close();
    }
  }
  
  private async getCanonicalUrl(videoSrc: string): Promise<string> {
    try {
      const response = await fetch(videoSrc, { method: 'HEAD' });
      return response.url || videoSrc;
    } catch (error) {
      console.warn('Could not get canonical URL, using original source:', error);
      return videoSrc;
    }
  }
  
  private async downloadVideo(url: string, outputPath: string): Promise<number> {
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to fetch video: ${response.statusText}`);
    }
    
    if (!response.body) {
      throw new Error('No response body');
    }
    
    const writeStream = fs.createWriteStream(outputPath);
    await pipeline(Readable.fromWeb(response.body as NodeReadableStream), writeStream);
    
    const stats = await fs.promises.stat(outputPath);
    return stats.size;
  }
}

export class DownloadService {
  private queue: DownloadJob[] = [];
  private completed: CompletedDownload[] = [];
  private processing = false;
  private sxyPrnDownloader = new SxyPrnDownloader();

  enqueue(url: string, name: string): string {
    const jobId = this.generateJobId();
    const job: DownloadJob = {
      jobId,
      url,
      name,
      enqueuedAt: new Date(),
    };
    this.queue.push(job);
    this.processQueue();
    return jobId;
  }

  getCompleted(): CompletedDownload[] {
    return [...this.completed];
  }

  private generateJobId(): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let result = '';
    for (let i = 0; i < 8; i++) {
      result += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return result;
  }

  private async processQueue(): Promise<void> {
    if (this.processing) return;
    this.processing = true;

    while (this.queue.length > 0) {
      const job = this.queue.shift()!;
      await this.processJob(job);
    }

    this.processing = false;
  }

  private async processJob(job: DownloadJob): Promise<void> {
    try {
      const urlObj = new URL(job.url);
      let downloader: Downloader;
      
      if (urlObj.hostname.includes('sxyprn.com')) {
        downloader = this.sxyPrnDownloader;
      } else {
        throw new Error('Not implemented');
      }
      
      const result = await downloader.download(job.url, job.name);
      this.completed.push(result);
    } catch (error) {
      console.error(`Failed to process job ${job.jobId}:`, error);
    }
  }
}