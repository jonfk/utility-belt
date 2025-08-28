import type { CompletedDownload } from '../schemas.js';
import fs from 'fs';
import path from 'path';
import { pipeline } from 'stream/promises';
import { Readable } from 'stream';
import type { ReadableStream as NodeReadableStream } from 'stream/web';
import { EventEmitter } from 'events';
import type { FastifyBaseLogger } from 'fastify';
import { 
  UnsupportedUrlError, 
  VideoSourceNotFoundError, 
  NetworkError,
  DownloadFailedError 
} from '../errors.js';
import { getBrowser } from '../puppeteer.js';

export interface DownloadJob {
  jobId: string;
  url: string;
  name: string;
  enqueuedAt: Date;
}

interface Downloader {
  download(jobId: string, url: string, name: string): Promise<CompletedDownload>;
}

class SxyPrnDownloader implements Downloader {
  private logger: FastifyBaseLogger;

  constructor(logger: FastifyBaseLogger) {
    this.logger = logger;
  }

  async download(jobId: string, url: string, name: string): Promise<CompletedDownload> {
    const jobLogger = this.logger.child({ jobId, url, name });
    jobLogger.info('Starting video download from SxyPrn');
    
    const browser = await getBrowser();
    const page = await browser.newPage();
    
    try {
      jobLogger.debug('Navigating to video page');
      await page.goto(url, { waitUntil: 'networkidle0' });
      
      jobLogger.debug('Waiting for video player element');
      await page.waitForSelector('#player_el');
      
      jobLogger.debug('Extracting video source URL');
      const videoSrc = await page.$eval('#player_el', el => (el as HTMLVideoElement).src);
      if (!videoSrc) {
        throw new VideoSourceNotFoundError('Could not find video source on page');
      }
      
      jobLogger.info({ videoSrc }, 'Found video source, resolving canonical URL');
      const canonicalUrl = await this.getCanonicalUrl(videoSrc);
      const outputPath = path.join(process.cwd(), 'data', `${name}.mp4`);
      
      jobLogger.debug({ outputPath }, 'Creating output directory');
      await fs.promises.mkdir(path.dirname(outputPath), { recursive: true });
      
      const startedAt = new Date();
      jobLogger.info({ canonicalUrl, outputPath }, 'Starting video file download');
      const size = await this.downloadVideo(canonicalUrl, outputPath);
      const finishedAt = new Date();
      
      const downloadDuration = finishedAt.getTime() - startedAt.getTime();
      jobLogger.info({ 
        outputPath,
        size, 
        downloadDuration 
      }, 'Video download completed successfully');
      
      return {
        url,
        name,
        savedPath: outputPath,
        size,
        startedAt: startedAt.toISOString(),
        finishedAt: finishedAt.toISOString()
      };
    } catch (error) {
      jobLogger.error({ error }, 'Video download failed');
      if (error instanceof VideoSourceNotFoundError) {
        throw error;
      }
      throw new DownloadFailedError(`Failed to download video from ${url}`, { 
        originalError: error instanceof Error ? error.message : String(error) 
      });
    } finally {
      await page.close();
    }
  }
  
  private async getCanonicalUrl(videoSrc: string): Promise<string> {
    try {
      const response = await fetch(videoSrc, { method: 'HEAD' });
      return response.url || videoSrc;
    } catch (error) {
      throw new NetworkError('Could not resolve canonical URL for video source', {
        videoSrc,
        originalError: error instanceof Error ? error.message : String(error)
      });
    }
  }
  
  private async downloadVideo(url: string, outputPath: string): Promise<number> {
    try {
      const response = await fetch(url);
      if (!response.ok) {
        throw new NetworkError(`Failed to fetch video: ${response.status} ${response.statusText}`, {
          url,
          status: response.status,
          statusText: response.statusText
        });
      }
      
      if (!response.body) {
        throw new NetworkError('Response body is empty', { url });
      }
      
      const writeStream = fs.createWriteStream(outputPath);
      await pipeline(Readable.fromWeb(response.body as NodeReadableStream), writeStream);
      
      const stats = await fs.promises.stat(outputPath);
      return stats.size;
    } catch (error) {
      if (error instanceof NetworkError) {
        throw error;
      }
      throw new DownloadFailedError('Failed to download and save video file', {
        url,
        outputPath,
        originalError: error instanceof Error ? error.message : String(error)
      });
    }
  }
}

export class DownloadService extends EventEmitter {
  private queue: DownloadJob[] = [];
  private completed: CompletedDownload[] = [];
  private sxyPrnDownloader: SxyPrnDownloader;
  private logger: FastifyBaseLogger;

  constructor(logger: FastifyBaseLogger) {
    super();
    this.logger = logger;
    this.sxyPrnDownloader = new SxyPrnDownloader(logger);
    this.startProcessor();
  }

  enqueue(url: string, name: string): string {
    const jobId = this.generateJobId();
    const job: DownloadJob = {
      jobId,
      url,
      name,
      enqueuedAt: new Date(),
    };
    this.queue.push(job);
    
    this.logger.info({
      jobId,
      url,
      name,
      queueLength: this.queue.length
    }, 'Download job enqueued');
    
    this.emit('jobAdded');
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

  private async startProcessor(): Promise<void> {
    this.processLoop().catch(error => {
      this.logger.error({ error }, 'Download processor crashed');
    });
  }

  private async processLoop(): Promise<void> {
    while (true) {
      if (this.queue.length === 0) {
        await new Promise<void>(resolve => this.once('jobAdded', resolve));
        continue;
      }

      const job = this.queue.shift()!;
      await this.processJob(job);
    }
  }

  private async processJob(job: DownloadJob): Promise<void> {
    const startTime = Date.now();
    
    this.logger.info({
      jobId: job.jobId,
      url: job.url,
      name: job.name,
      enqueuedAt: job.enqueuedAt,
      waitTime: startTime - job.enqueuedAt.getTime()
    }, 'Starting download job processing');
    
    try {
      const urlObj = new URL(job.url);
      let downloader: Downloader;
      
      if (urlObj.hostname.includes('sxyprn.com')) {
        downloader = this.sxyPrnDownloader;
      } else {
        throw new UnsupportedUrlError(`URL hostname not supported: ${urlObj.hostname}`);
      }
      
      const result = await downloader.download(job.jobId, job.url, job.name);
      this.completed.push(result);
      
      const endTime = Date.now();
      const processingTime = endTime - startTime;
      
      this.logger.info({
        jobId: job.jobId,
        url: job.url,
        name: job.name,
        savedPath: result.savedPath,
        size: result.size,
        processingTime,
        startedAt: result.startedAt,
        finishedAt: result.finishedAt
      }, 'Download job completed successfully');
      
    } catch (error) {
      const endTime = Date.now();
      const processingTime = endTime - startTime;
      
      this.logger.error({ 
        jobId: job.jobId, 
        url: job.url, 
        name: job.name,
        processingTime,
        error 
      }, 'Failed to process download job');
    }
  }
}